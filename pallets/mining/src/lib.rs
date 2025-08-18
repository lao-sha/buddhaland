#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, EnsureOrigin, Get},
    };
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_runtime::{RuntimeDebug, traits::Saturating};
    use core::convert::TryInto;

    /// 会话质量摘要（调用方传入的最小必要指标）
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct SessionQuality {
        /// 是否通过基础有效性校验（如 poor_signal 比例门槛、时间长度阈值）
        pub valid: bool,
        /// 有效分钟数（按阈值过滤后的有效时长）
        pub valid_minutes: u16,
        /// 冥想质量系数（百分比 0..100）
        pub quality_pct: u8,
    }

    /// 运行时关联余额
    pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// 授权适配接口：由 runtime 实现，桥接到实际的授权中心
    pub trait MiningAuthorizer<AccountId> {
        /// 调用方是否被授权为“记账/发奖模块账户”
        fn is_authorized(caller: &AccountId) -> bool;
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// 事件类型
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// 发放 BUD 的代币接口
        type Currency: Currency<Self::AccountId>;
        /// 管理员 Origin（用于参数治理）
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        /// 授权适配器（白名单调用者为“记账/发奖模块账户”）
        type Authorizer: MiningAuthorizer<Self::AccountId>;

        /// 单会话有效分钟上限（防异常大值）
        type MaxSessionMinutes: Get<u16>;
        /// 单设备每日发放上限（以 BUD 计）
        type DailyCapPerDevice: Get<BalanceOf<Self>>;
        /// 单账户每日发放上限
        type DailyCapPerAccount: Get<BalanceOf<Self>>;
        /// 全网每日总发放上限
        type DailyCapGlobal: Get<BalanceOf<Self>>;
        /// 基础每分钟奖励（BUD）
        type BaseBudPerMinute: Get<BalanceOf<Self>>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// 设备当日已发放（设备维度限流）
    #[pallet::storage]
    #[pallet::getter(fn device_daily_minted)]
    pub type DeviceDailyMinted<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat, H256,               // device_id
        Blake2_128Concat, u32,                // day index
        BalanceOf<T>, ValueQuery
    >;

    /// 账户当日已发放（账户维度限流）
    #[pallet::storage]
    #[pallet::getter(fn account_daily_minted)]
    pub type AccountDailyMinted<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat, T::AccountId,       // user
        Blake2_128Concat, u32,                // day index
        BalanceOf<T>, ValueQuery
    >;

    /// 全网当日已发放（全局维度限流）
    #[pallet::storage]
    #[pallet::getter(fn global_daily_minted)]
    pub type GlobalDailyMinted<T: Config> = StorageMap<
        _, Blake2_128Concat, u32, BalanceOf<T>, ValueQuery
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// 发奖成功
        Mined { who: T::AccountId, device_id: H256, amount: BalanceOf<T>, day_index: u32 },
    }

    #[pallet::error]
    pub enum Error<T> {
        NotAuthorized,
        ExceedDailyCapDevice,
        ExceedDailyCapAccount,
        ExceedDailyCapGlobal,
        InvalidQuality,
    }

    impl<T: Config> Pallet<T> {
        /// 简单将区块高度映射为“天索引”（约等于自然日，具体依赖每块时间配置）
        fn day_index(now: BlockNumberFor<T>) -> u32 {
            // 以 14400 块/天（6s 区块）估算：86400/6 = 14400
            let blocks_per_day: u32 = 14400;
            let n: u32 = TryInto::<u32>::try_into(now).unwrap_or(0);
            n / blocks_per_day
        }

        /// 校验调用者是否在授权中心白名单（命名空间 + 账户）
        fn ensure_authorized(caller: &T::AccountId) -> Result<(), Error<T>> {
            if <T as Config>::Authorizer::is_authorized(caller) { Ok(()) } else { Err(Error::<T>::NotAuthorized) }
        }
    }

    /// 供其他 Pallet 调用的发奖接口（内部 API）
    pub trait MiningInterface<AccountId, Balance> {
        /// 直接记账并发奖（绕过 extrinsic origin），需要调用方提供“白名单模块账户”作为 caller
        fn award_by(
            caller: &AccountId,
            who: &AccountId,
            device_id: H256,
            valid: bool,
            valid_minutes: u16,
            quality_pct: u8,
        ) -> DispatchResult;
    }

    impl<T: Config> MiningInterface<T::AccountId, BalanceOf<T>> for Pallet<T> {
        /// 发奖内部实现：与 `mine` 相同的限流与发放逻辑
        fn award_by(
            caller: &T::AccountId,
            who: &T::AccountId,
            device_id: H256,
            valid: bool,
            valid_minutes: u16,
            quality_pct: u8,
        ) -> DispatchResult {
            // 授权校验
            Self::ensure_authorized(caller)?;
            ensure!(valid, Error::<T>::InvalidQuality);
            let minutes = core::cmp::min(valid_minutes, T::MaxSessionMinutes::get());

            let base = T::BaseBudPerMinute::get();
            let mut amount = base.saturating_mul(<BalanceOf<T>>::from(minutes as u32));
            amount = amount.saturating_mul(<BalanceOf<T>>::from(quality_pct as u32));
            amount = amount / <BalanceOf<T>>::from(100u32);

            let now = <frame_system::Pallet<T>>::block_number();
            let day = Self::day_index(now);

            let dev_used = DeviceDailyMinted::<T>::get(device_id, day);
            ensure!(dev_used.saturating_add(amount) <= T::DailyCapPerDevice::get(), Error::<T>::ExceedDailyCapDevice);
            let acc_used = AccountDailyMinted::<T>::get(who, day);
            ensure!(acc_used.saturating_add(amount) <= T::DailyCapPerAccount::get(), Error::<T>::ExceedDailyCapAccount);
            let g_used = GlobalDailyMinted::<T>::get(day);
            ensure!(g_used.saturating_add(amount) <= T::DailyCapGlobal::get(), Error::<T>::ExceedDailyCapGlobal);

            DeviceDailyMinted::<T>::insert(device_id, day, dev_used.saturating_add(amount));
            AccountDailyMinted::<T>::insert(who, day, acc_used.saturating_add(amount));
            GlobalDailyMinted::<T>::insert(day, g_used.saturating_add(amount));

            <T as Config>::Currency::deposit_creating(who, amount);
            Self::deposit_event(Event::Mined { who: who.clone(), device_id, amount, day_index: day });
            Ok(())
        }
    }
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// 记账并发放奖励（由白名单模块账户/服务调用）
        /// - 参数：用户、设备、会话质量
        /// - 限制：设备/账户/全网三级限流
        /// - 奖励：base_per_min × valid_minutes × quality_pct/100
        #[pallet::call_index(0)]
        #[pallet::weight(10_000)]
        pub fn mine(
            origin: OriginFor<T>,
            who: T::AccountId,
            device_id: H256,
            valid: bool,
            valid_minutes: u16,
            quality_pct: u8,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            Self::ensure_authorized(&caller)?;

            ensure!(valid, Error::<T>::InvalidQuality);
            let minutes = core::cmp::min(valid_minutes, T::MaxSessionMinutes::get());

            let base = T::BaseBudPerMinute::get();
            // amount = base * minutes * quality_pct / 100
            let mut amount = base.saturating_mul(<BalanceOf<T>>::from(minutes as u32));
            amount = amount.saturating_mul(<BalanceOf<T>>::from(quality_pct as u32));
            // 简单除法：按 100 下取整
            amount = amount / <BalanceOf<T>>::from(100u32);

            // day index
            let now = <frame_system::Pallet<T>>::block_number();
            let day = Self::day_index(now);

            // 限流校验：设备
            let dev_used = DeviceDailyMinted::<T>::get(device_id, day);
            ensure!(dev_used.saturating_add(amount) <= T::DailyCapPerDevice::get(), Error::<T>::ExceedDailyCapDevice);
            // 账户
            let acc_used = AccountDailyMinted::<T>::get(&who, day);
            ensure!(acc_used.saturating_add(amount) <= T::DailyCapPerAccount::get(), Error::<T>::ExceedDailyCapAccount);
            // 全网
            let g_used = GlobalDailyMinted::<T>::get(day);
            ensure!(g_used.saturating_add(amount) <= T::DailyCapGlobal::get(), Error::<T>::ExceedDailyCapGlobal);

            // 记账
            DeviceDailyMinted::<T>::insert(device_id, day, dev_used.saturating_add(amount));
            AccountDailyMinted::<T>::insert(&who, day, acc_used.saturating_add(amount));
            GlobalDailyMinted::<T>::insert(day, g_used.saturating_add(amount));

            // 发放 BUD（直接 mint 到用户，或从奖励池账户转账：此处直接增加余额）
            <T as Config>::Currency::deposit_creating(&who, amount);

            Self::deposit_event(Event::Mined { who, device_id, amount, day_index: day });
            Ok(())
        }
    }
}


