#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		pallet_prelude::*,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::{RuntimeDebug, traits::{AtLeast32BitUnsigned, SaturatedConversion, Saturating}};
	use alloc::vec::Vec;
	use pallet_authorizer::pallet::Namespace;

	/// 业力记录事件类型：获取或消费
	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub enum KarmaEventType {
		Gain,
		Spend,
	}

	/// 业力变动记录结构
	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(S))]
	pub struct KarmaRecord<Balance, Moment, S: frame_support::traits::Get<u32>> {
		/// 事件类型（获取/消费）
		pub event_type: KarmaEventType,
		/// 本次变动的数量
		pub amount: Balance,
		/// 变动后账户 Karma 余额
		pub total_karma_after: Balance,
		/// 变动后账户累计功德值
		pub total_merit_after: Balance,
		/// 变动后账户修为等级
		pub level_after: u32,
		/// 记录时间（区块高度）
		pub timestamp: Moment,
		/// 备注信息（如来源、说明）
		pub memo: BoundedVec<u8, S>,
	}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Runtime 事件类型
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Karma/功德的余额类型
		type KarmaBalance: Parameter + AtLeast32BitUnsigned + Default + Copy + MaxEncodedLen;
		/// 每个账户历史记录的最大长度
		type HistoryMaxLen: Get<u32>;
		/// 备注字段的最大长度（字节）
		type MaxMemoLen: Get<u32>;
		/// 授权中心命名空间（例如以 Karma 的 PalletId 字节为命名空间）
		type AuthorizerNamespace: Get<[u8; 8]>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn karma_of)]
	pub type KarmaOf<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, T::KarmaBalance, ValueQuery>;

	/// 账户的累计功德值（终身累计，来源于消费行为）
	#[pallet::storage]
	#[pallet::getter(fn total_merit_of)]
	pub type TotalMeritOf<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, T::KarmaBalance, ValueQuery>;

	/// 黑洞地址累计（全网被消费的 Karma 总量统计）
	#[pallet::storage]
	#[pallet::getter(fn total_burned)]
	pub type TotalBurned<T: Config> = StorageValue<_, T::KarmaBalance, ValueQuery>;

	/// 账户的修为等级（由累计功德值推导得到）
	#[pallet::storage]
	#[pallet::getter(fn level_of)]
	pub type LevelOf<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

	// 废弃内置白名单，改为外部授权中心（pallet-authorizer）

	/// 账户 Karma 历史记录（仅追加）
	#[pallet::storage]
	#[pallet::getter(fn history_of)]
	pub type HistoryOf<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<KarmaRecord<T::KarmaBalance, BlockNumberFor<T>, T::MaxMemoLen>, T::HistoryMaxLen>,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		KarmaGained { account: T::AccountId, amount: T::KarmaBalance, new_balance: T::KarmaBalance },
		KarmaSpent {
			account: T::AccountId,
			amount: T::KarmaBalance,
			new_balance: T::KarmaBalance,
			new_total_merit: T::KarmaBalance,
			new_level: u32,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		NotAuthorized,
		InsufficientKarma,
		HistoryOverflow,
		/// 备注太长，超过 `MaxMemoLen`
		MemoTooLong,
	}

	// 移除 Root/Sudo 内置授权维护接口，统一由 pallet-authorizer 治理

	impl<T: Config> Pallet<T> {
		/// 通过授权中心校验是否授权（命名空间 + 调用账户）
		fn ensure_authorized(who: &T::AccountId) -> Result<(), Error<T>>
		where
			T: pallet_authorizer::Config,
		{
			let ns = Namespace(T::AuthorizerNamespace::get());
			if pallet_authorizer::Pallet::<T>::is_authorized(ns, who) { Ok(()) } else { Err(Error::<T>::NotAuthorized) }
		}

		/// 追加一条历史记录（超出上限时返回错误）
		fn push_history(
			account: &T::AccountId,
			record: KarmaRecord<T::KarmaBalance, BlockNumberFor<T>, T::MaxMemoLen>,
		) -> Result<(), Error<T>> {
			HistoryOf::<T>::try_mutate(account, |hist| {
				if hist.try_push(record).is_err() { return Err(Error::<T>::HistoryOverflow); }
				Ok(())
			})
		}

		fn recalc_level(total_merit: T::KarmaBalance) -> u32 {
			// 等级规则（示例）：阈值阶梯式增长
			// 0..100 -> LV1, 100..300 -> LV2, 300..700 -> LV3 ...
			// 每跨越一个阈值提升一级，阈值按（翻倍 + 100）增长，上限 100 级
			let mut level: u32 = 0;
			let mut threshold: u128 = 100;
			let tm: u128 = total_merit.saturated_into::<u128>();
			while tm >= threshold {
				level = level.saturating_add(1);
				threshold = threshold.saturating_add(threshold + 100);
				if level >= 100 { break; }
			}
			level
		}
	}

	/// 供其他 Pallet 调用的外部接口（增发/消费 Karma）
	pub trait KarmaCurrency<AccountId> {
		type Balance;
		/// 为账户增加 Karma（需授权调用者）
		fn gain(origin_caller: &AccountId, who: &AccountId, amount: Self::Balance, memo: Vec<u8>) -> DispatchResult;
		/// 为账户消费 Karma（需授权且余额充足），并累计功德值与等级
		fn spend(origin_caller: &AccountId, who: &AccountId, amount: Self::Balance, memo: Vec<u8>) -> DispatchResult;
	}

	impl<T: Config + pallet_authorizer::Config> KarmaCurrency<T::AccountId> for Pallet<T> {
		type Balance = T::KarmaBalance;

		/// 增加 Karma：白名单校验 -> 更新余额 -> 写入历史 -> 事件
		fn gain(
			origin_caller: &T::AccountId,
			who: &T::AccountId,
			amount: Self::Balance,
			memo: Vec<u8>,
		) -> DispatchResult {
			Self::ensure_authorized(origin_caller)?;
			KarmaOf::<T>::mutate(who, |bal| {
				*bal = (*bal).saturating_add(amount);
			});
			let new_balance = KarmaOf::<T>::get(who);
			let now: BlockNumberFor<T> = <frame_system::Pallet<T>>::block_number();
			let memo_bounded: BoundedVec<u8, T::MaxMemoLen> = BoundedVec::try_from(memo)
				.map_err(|_| Error::<T>::MemoTooLong)?;
			let rec = KarmaRecord {
				event_type: KarmaEventType::Gain,
				amount,
				total_karma_after: new_balance,
				total_merit_after: TotalMeritOf::<T>::get(who),
				level_after: LevelOf::<T>::get(who),
				timestamp: now,
				memo: memo_bounded,
			};
			Self::push_history(who, rec)?;
			Self::deposit_event(Event::KarmaGained { account: who.clone(), amount, new_balance });
			Ok(())
		}

		/// 消费 Karma：白名单校验 -> 扣减余额 -> 累计功德与等级 -> 写历史 -> 事件
		fn spend(
			origin_caller: &T::AccountId,
			who: &T::AccountId,
			amount: Self::Balance,
			memo: Vec<u8>,
		) -> DispatchResult {
			Self::ensure_authorized(origin_caller)?;
			KarmaOf::<T>::try_mutate(who, |bal| -> Result<(), Error<T>> {
				if *bal < amount { return Err(Error::<T>::InsufficientKarma); }
				*bal = (*bal).saturating_sub(amount);
				Ok(())
			})?;
			TotalMeritOf::<T>::mutate(who, |m| *m = (*m).saturating_add(amount));
			// 消费即“打入黑洞”：累加全局 TotalBurned
			TotalBurned::<T>::mutate(|b| *b = (*b).saturating_add(amount));
			let total_merit = TotalMeritOf::<T>::get(who);
			let new_level = Self::recalc_level(total_merit);
			LevelOf::<T>::insert(who, new_level);
			let new_balance = KarmaOf::<T>::get(who);
			let now: BlockNumberFor<T> = <frame_system::Pallet<T>>::block_number();
			let memo_bounded: BoundedVec<u8, T::MaxMemoLen> = BoundedVec::try_from(memo)
				.map_err(|_| Error::<T>::MemoTooLong)?;
			let rec = KarmaRecord {
				event_type: KarmaEventType::Spend,
				amount,
				total_karma_after: new_balance,
				total_merit_after: total_merit,
				level_after: new_level,
				timestamp: now,
				memo: memo_bounded,
			};
			Self::push_history(who, rec)?;
			Self::deposit_event(Event::KarmaSpent {
				account: who.clone(),
				amount,
				new_balance,
				new_total_merit: total_merit,
				new_level,
			});
			Ok(())
		}
	}
}


