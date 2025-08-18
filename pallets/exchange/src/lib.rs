#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

// 使 weights.rs 成为 crate 根模块（避免在 pallet 模块中声明导致的 proc-macro 限制）
pub mod weights;

pub use self::pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, ExistenceRequirement, Get},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::{AccountIdConversion, Saturating, UniqueSaturatedInto, Zero};
	use sp_runtime::RuntimeDebug;
	use alloc::{vec::Vec, collections::BTreeSet};
    use crate::weights::WeightInfo;
    use pallet_karma::KarmaCurrency;

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	/// 分配项输入（用于批量设置）
	#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen, RuntimeDebug)]
	pub struct AllocInput {
		/// 子账户ID（用于 PalletId.into_sub_account_truncating 的种子，8字节）
		pub id: [u8; 8],
		/// 该子账户的分配比例，基点(BPS)
		pub bps: u16,
		/// 是否启用
		pub enabled: bool,
	}

	/// 分配项存储结构
	#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen, Default)]
	pub struct Allocation {
		/// 基点(BPS)
		pub bps: u16,
		/// 是否启用
		pub enabled: bool,
	}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// 事件类型
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// BUD 代币货币接口
		type Currency: Currency<Self::AccountId>;
		/// PalletId（用于派生模块主账户与子账户）
		type PalletIdGet: Get<PalletId>;
		/// BPS 分母（通常 10000）
		type BpsDenominator: Get<u16>;
		/// 分配项最大数量
		type MaxAllocs: Get<u32>;
		/// 备注最大长度
		type MaxMemoLen: Get<u32>;
		/// 用于分配管理的授权命名空间（pallet-authorizer）
		type AdminAuthorizerNs: Get<[u8; 8]>;
		/// 管理员校验适配器（由 runtime 实现，内部可桥接 pallet-authorizer）
		type Admin: AdminAuthorizer<Self::AccountId>;
		/// 权重接口
		type WeightInfo: WeightInfo;
		/// Karma 适配器（由 runtime 绑定到 pallet_karma::Pallet<Runtime>）
		type Karma: pallet_karma::pallet::KarmaCurrency<Self::AccountId, Balance = BalanceOf<Self>>;
	}

	/// 管理员校验接口（低耦合）：由 runtime 提供实现
	pub trait AdminAuthorizer<AccountId> {
		fn is_admin(who: &AccountId) -> bool;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// 分配项ID列表（保持顺序）
	#[pallet::storage]
	#[pallet::getter(fn alloc_ids)]
	pub type AllocIds<T: Config> = StorageValue<_, BoundedVec<[u8; 8], T::MaxAllocs>, ValueQuery>;

	/// 分配项详情
	#[pallet::storage]
	#[pallet::getter(fn alloc_of)]
	pub type Allocs<T: Config> = StorageMap<_, Blake2_128Concat, [u8; 8], Allocation, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// 批量设置完成（替换式）
		AllocsReplaced { count: u32 },
		/// 更新单个分配项
		AllocUpdated { id: [u8; 8], bps: u16, enabled: bool },
		/// 移除分配项
		AllocRemoved { id: [u8; 8] },
		/// 兑换成功
		Exchanged { who: T::AccountId, bud_in: BalanceOf<T>, minted_karma: BalanceOf<T>, recipients: u32 },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// 无权限（需在授权白名单中）
		NotAuthorized,
		/// 分配项重复
		DuplicateAllocId,
		/// 分配项不存在
		AllocNotFound,
		/// 分配项超出上限
		TooManyAllocs,
		/// 启用分配项BPS之和不等于分母
		InvalidBpsSum,
		/// 金额为零
		ZeroAmount,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// 函数级中文注释：
		/// 批量替换分配项（创建/更新/删除的统一接口）。
		/// - 仅授权账户可调用（通过 pallet-authorizer 的 `AdminAuthorizerNs`）
		/// - 所有“启用中的”分配项 BPS 之和必须等于分母 `BpsDenominator`
		/// - 传入列表为最终生效集合（会覆盖原有分配状态）
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_allocs(items.len() as u32))]
		pub fn set_allocs(origin: OriginFor<T>, items: BoundedVec<([u8;8], u16, bool), T::MaxAllocs>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::ensure_admin(&who)?;

			// 校验重复与BPS和
			let mut seen: BTreeSet<[u8;8]> = BTreeSet::new();
			let mut sum_enabled: u32 = 0;
			for &(id, bps, enabled) in items.iter() {
				ensure!(seen.insert(id), Error::<T>::DuplicateAllocId);
				if enabled { sum_enabled = sum_enabled.saturating_add(bps as u32); }
			}
			ensure!(sum_enabled as u16 == T::BpsDenominator::get(), Error::<T>::InvalidBpsSum);

			// 覆盖式写入
			let mut ids: BoundedVec<[u8; 8], T::MaxAllocs> = BoundedVec::default();
			for (id, bps, enabled) in items.into_iter() {
				let alloc = Allocation { bps, enabled };
				Allocs::<T>::insert(id, alloc);
				ids.try_push(id).map_err(|_| Error::<T>::TooManyAllocs)?;
			}
			AllocIds::<T>::put(&ids);

			Self::deposit_event(Event::AllocsReplaced { count: ids.len() as u32 });
			Ok(())
		}

		/// 函数级中文注释：
		/// 更新单个分配项（不存在则报错）。
		/// - 为保证“随时保持总和等于分母”，本函数调用后会校验全部启用项之和。
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::set_allocs(1))]
		pub fn update_alloc(origin: OriginFor<T>, id: [u8; 8], bps: u16, enabled: bool) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::ensure_admin(&who)?;

			ensure!(Allocs::<T>::contains_key(id), Error::<T>::AllocNotFound);
			Allocs::<T>::insert(id, Allocation { bps, enabled });

			// 校验BPS总和
			Self::ensure_bps_sum_valid()?;

			Self::deposit_event(Event::AllocUpdated { id, bps, enabled });
			Ok(())
		}

		/// 函数级中文注释：
		/// 移除分配项。移除后仍需满足启用项BPS和等于分母，否则拒绝。
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::set_allocs(1))]
		pub fn remove_alloc(origin: OriginFor<T>, id: [u8; 8]) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::ensure_admin(&who)?;

			ensure!(Allocs::<T>::contains_key(id), Error::<T>::AllocNotFound);
			Allocs::<T>::remove(id);
			AllocIds::<T>::mutate(|list| {
				if let Some(pos) = list.iter().position(|x| x == &id) { list.swap_remove(pos); }
			});

			// 校验BPS总和
			Self::ensure_bps_sum_valid()?;

			Self::deposit_event(Event::AllocRemoved { id });
			Ok(())
		}

		/// 函数级中文注释：
		/// 进行 BUD→Karma 兑换：
		/// - 从调用者账户按当前“启用的子账户分配”划转 BUD 到对应“PalletId 子账户”
		/// - 用本模块主账户作为“调用者”调用 `pallet-karma::gain` 为调用者增发同额 Karma
		/// - 备注 `memo` 会写入 Karma 历史
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::exchange(0))]
		pub fn exchange(
			origin: OriginFor<T>,
			amount: BalanceOf<T>,
			memo: BoundedVec<u8, T::MaxMemoLen>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);

			// 收集启用项
			let ids = AllocIds::<T>::get();
			let mut enabled: Vec<([u8; 8], u16)> = Vec::new();
			for id in ids.into_iter() {
				if let Some(a) = Allocs::<T>::get(id) { if a.enabled && a.bps > 0 { enabled.push((id, a.bps)); } }
			}
			// 校验BPS总和（兑换前再校验一次）
			let sum_bps: u32 = enabled.iter().fold(0u32, |acc, x| acc.saturating_add(x.1 as u32));
			ensure!(sum_bps as u16 == T::BpsDenominator::get(), Error::<T>::InvalidBpsSum);

			// 按BPS比例转账，尾差给第一个
			let denom = T::BpsDenominator::get() as u128;
			let amount_u128: u128 = amount.unique_saturated_into();
			let mut total_distributed: BalanceOf<T> = Zero::zero();
			for &(id, bps) in enabled.iter() {
				let share_u128: u128 = amount_u128.saturating_mul(bps as u128) / denom;
				let share: BalanceOf<T> = share_u128.unique_saturated_into();
				// 划转
				let dest = Self::sub_account(id);
				<T as Config>::Currency::transfer(&who, &dest, share, ExistenceRequirement::KeepAlive)?;
				total_distributed = total_distributed.saturating_add(share);
			}
			// 尾差补到第一位
			let remainder = amount.saturating_sub(total_distributed);
			if !remainder.is_zero() {
				if let Some((first_id, _)) = enabled.first() {
					let dest = Self::sub_account(*first_id);
					<T as Config>::Currency::transfer(&who, &dest, remainder, ExistenceRequirement::KeepAlive)?;
				}
			}

			// 增发Karma：以模块主账户为“调用者”身份
			let caller = Self::module_account();
			let memo_vec: Vec<u8> = memo.into_inner().to_vec();
			<T as Config>::Karma::gain(&caller, &who, amount, memo_vec)?;

			Self::deposit_event(Event::Exchanged { who, bud_in: amount, minted_karma: amount, recipients: enabled.len() as u32 });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// 函数级中文注释：
		/// 返回本模块主账户（由 PalletId 派生），作为调用 Karma 的“授权调用者”
		pub fn module_account() -> T::AccountId { T::PalletIdGet::get().into_account_truncating() }

		/// 函数级中文注释：
		/// 根据分配项ID（8字节）派生对应子账户，用于收款
		pub fn sub_account(id: [u8; 8]) -> T::AccountId { T::PalletIdGet::get().into_sub_account_truncating(id) }

		/// 函数级中文注释：
		/// 统计当前启用的分配项数量
		pub fn enabled_allocs_count() -> u32 {
			let mut n = 0u32;
			for id in AllocIds::<T>::get().into_iter() {
				if let Some(a) = Allocs::<T>::get(id) { if a.enabled && a.bps > 0 { n = n.saturating_add(1); } }
			}
			n
		}

		/// 函数级中文注释：
		/// 校验所有启用项BPS总和是否等于分母
		fn ensure_bps_sum_valid() -> DispatchResult {
			let mut sum: u32 = 0;
			for id in AllocIds::<T>::get().into_iter() {
				if let Some(a) = Allocs::<T>::get(id) { if a.enabled { sum = sum.saturating_add(a.bps as u32); } }
			}
			ensure!(sum as u16 == T::BpsDenominator::get(), Error::<T>::InvalidBpsSum);
			Ok(())
		}

		/// 函数级中文注释：
		/// 通过 Admin 适配器校验调用账户是否在 Exchange 管理命名空间白名单
		fn ensure_admin(who: &T::AccountId) -> Result<(), Error<T>> {
			if <T as Config>::Admin::is_admin(who) { Ok(()) } else { Err(Error::<T>::NotAuthorized) }
		}
	}
}


