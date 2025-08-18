#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, EnsureOrigin, ReservableCurrency},
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::{RuntimeDebug, traits::{Saturating, Zero}};
	use alloc::vec::Vec;
    use frame_support::traits::GenesisBuild;

	/// 命名空间类型：使用固定 8 字节标识（可用 PalletId 的 bytes）
	#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct Namespace(pub [u8; 8]);

	/// 投票选项
	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub enum VoteChoice { Aye, Nay }

	/// 提案操作：增加或移除授权账户
	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub enum Op { Add, Remove }

	/// 提案结构（最小骨架版）
	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct Proposal<AccountId, Balance, BlockNumber> {
		pub ns: Namespace,
		pub op: Op,
		pub target: AccountId,
		pub proposer: AccountId,
		pub deposit: Balance,
		pub end: BlockNumber,
		pub aye: Balance,
		pub nay: Balance,
		pub executed: bool,
	}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// 事件类型
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// 治理代币（BUD）
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
		/// 每个提案的最小押注
		type MinDeposit: Get<BalanceOf<Self>>;
		/// 投票期（区块数）
		type VotingPeriod: Get<BlockNumberFor<Self>>;
		/// 管理 Origin（可选用于紧急取消/参数调整）
		type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;
	}

	pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn authorized)]
	/// 白名单：(命名空间, 账户) => ()
	pub type Authorized<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat, Namespace,
		Blake2_128Concat, T::AccountId,
		(), OptionQuery
	>;

	#[pallet::storage]
	#[pallet::getter(fn proposals)]
	/// 提案：自增 ID => Proposal
	pub type Proposals<T: Config> = StorageMap<
		_,
		Blake2_128Concat, u64,
		Proposal<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn next_proposal_id)]
	pub type NextProposalId<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn votes_of)]
	/// 记录某提案下某账户是否已投票，避免重复计票
	pub type VotesOf<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat, u64,
		Blake2_128Concat, T::AccountId,
		VoteChoice, OptionQuery
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// 新提案：命名空间 bytes、操作(0移除/1增加)
		ProposalSubmitted { id: u64, ns: [u8; 8], op: u8, target: T::AccountId, end: BlockNumberFor<T> },
		/// 投票：choice=1(赞成)/0(反对)
		Voted { id: u64, who: T::AccountId, choice: u8, weight: BalanceOf<T> },
		Executed { id: u64, success: bool },
	}

	#[pallet::error]
	pub enum Error<T> {
		ProposalNotFound,
		AlreadyExecuted,
		VotingClosed,
		AlreadyVoted,
		DepositTooLow,
		/// 非法操作码（应为 0/1）
		InvalidOp,
		/// 非法投票选项（应为 0/1）
		InvalidChoice,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// 发起授权变更提案（押注 >= MinDeposit）
		#[pallet::call_index(0)]
		#[pallet::weight(10_000)]
		pub fn submit_proposal(
			origin: OriginFor<T>,
			ns_bytes: [u8; 8],
			op_code: u8,
			target: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let min = T::MinDeposit::get();
			// 这里用保留押注，防 spam；最小骨架不赎回，完整版应在执行/过期后处理赎回
			T::Currency::reserve(&who, min).map_err(|_| Error::<T>::DepositTooLow)?;
			let id = NextProposalId::<T>::mutate(|x| { let id = *x; *x = x.saturating_add(1); id });
			let end = frame_system::Pallet::<T>::block_number() + T::VotingPeriod::get();
			let ns = Namespace(ns_bytes);
			let op = match op_code { 1 => Op::Add, 0 => Op::Remove, _ => return Err(Error::<T>::InvalidOp.into()) };
			let prop = Proposal { ns, op, target: target.clone(), proposer: who.clone(), deposit: min, end, aye: Zero::zero(), nay: Zero::zero(), executed: false };
			Proposals::<T>::insert(id, prop);
			Self::deposit_event(Event::ProposalSubmitted { id, ns: ns_bytes, op: op_code, target, end });
			Ok(())
		}

		/// 以账户持有的 BUD 余额为权重投票（快照=当前余额，最小骨架）
		#[pallet::call_index(1)]
		#[pallet::weight(10_000)]
		pub fn vote(origin: OriginFor<T>, id: u64, choice_code: u8) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(choice_code == 0 || choice_code == 1, Error::<T>::InvalidChoice);
			Proposals::<T>::try_mutate(id, |maybe| -> DispatchResult {
				let prop = maybe.as_mut().ok_or(Error::<T>::ProposalNotFound)?;
				ensure!(!prop.executed, Error::<T>::AlreadyExecuted);
				ensure!(frame_system::Pallet::<T>::block_number() <= prop.end, Error::<T>::VotingClosed);
				ensure!(VotesOf::<T>::get(id, &who).is_none(), Error::<T>::AlreadyVoted);
				let weight = T::Currency::free_balance(&who);
				let choice = if choice_code == 1 { VoteChoice::Aye } else { VoteChoice::Nay };
				match choice { VoteChoice::Aye => prop.aye = prop.aye.saturating_add(weight), VoteChoice::Nay => prop.nay = prop.nay.saturating_add(weight) };
				VotesOf::<T>::insert(id, &who, choice);
				Ok(())
			})?;
			let weight = T::Currency::free_balance(&who);
			Self::deposit_event(Event::Voted { id, who, choice: choice_code, weight });
			Ok(())
		}

		/// 到期后执行：简单多数通过即落地白名单变更
		#[pallet::call_index(2)]
		#[pallet::weight(10_000)]
		pub fn execute(origin: OriginFor<T>, id: u64) -> DispatchResult {
			let _ = ensure_signed(origin)?; // 任何人可触发执行（也可改为 AdminOrigin）
			Proposals::<T>::try_mutate(id, |maybe| -> DispatchResult {
				let prop = maybe.as_mut().ok_or(Error::<T>::ProposalNotFound)?;
				ensure!(!prop.executed, Error::<T>::AlreadyExecuted);
				ensure!(frame_system::Pallet::<T>::block_number() > prop.end, Error::<T>::VotingClosed);
				let passed = prop.aye > prop.nay;
				if passed {
					match prop.op {
						Op::Add => { Authorized::<T>::insert(prop.ns, &prop.target, ()); },
						Op::Remove => { Authorized::<T>::remove(prop.ns, &prop.target); },
					}
				}
				prop.executed = true;
				Ok(())
			})?;
			let success = true; // 最小骨架直接视为执行成功
			Self::deposit_event(Event::Executed { id, success });
			Ok(())
		}
	}

	// 本 MVP 移除 genesis 预置白名单，以避免对运行时构建的额外约束；
	// 后续需要可在创世后用交易初始化。

	impl<T: Config> Pallet<T> {
		/// 查询接口：命名空间+账户 是否已被授权
		pub fn is_authorized(ns: Namespace, who: &T::AccountId) -> bool {
			Authorized::<T>::contains_key(ns, who)
		}
	}
}


