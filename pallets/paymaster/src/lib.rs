#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::{
    dispatch::{DispatchResult, DispatchResultWithPostInfo, PostDispatchInfo, Pays},
    pallet_prelude::*,
    traits::{tokens::fungible::Mutate, Currency, ExistenceRequirement::AllowDeath, Get},
    PalletId,
};
use frame_system::pallet_prelude::*;
use sp_runtime::traits::{Saturating, Zero, AccountIdConversion};
use sp_std::vec::Vec;

use pallet_karma::{KarmaBalance};

pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum ProxyPermission {
        All,
        Transfer,
        Governance,
        Custom(Vec<u8>),
    }

    #[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
    pub struct ProxyConfig<AccountId, Balance> {
        pub proxy: AccountId,
        pub permission: ProxyPermission,
        pub fee_limit: Balance,
        pub enabled: bool,
    }

    #[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
    pub struct ThirdPartyPaymentRecord<Balance, BlockNumber> {
        pub amount: Balance,
        pub block_number: BlockNumber,
        pub timestamp: u64,
    }

    #[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
    pub struct DepositAuthorization<Balance> {
        pub max_amount: Option<Balance>,
        pub enabled: bool,
    }

    #[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
    pub struct FeeRecord<Balance, BlockNumber> {
        pub amount: Balance,
        pub block_number: BlockNumber,
        pub tx_hash: Option<[u8; 32]>,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type RuntimeCall: Parameter + Dispatchable<RuntimeOrigin = Self::RuntimeOrigin> + From<frame_system::Call<Self>>;
        /// Currency for BUD token
        type Currency: Currency<Self::AccountId>;
        /// Pallet Id based account
        #[pallet::constant]
        type PalletId: Get<PalletId>;
        /// Maximum batch size
        #[pallet::constant]
        type MaxBatchSize: Get<u32>;
        /// Minimum deposit amount
        #[pallet::constant]
        type MinimumDeposit: Get<BalanceOf<Self>>;
        /// Service fee rate (percent in basis points 0..=10000) collected into TotalServiceFees
        #[pallet::constant]
        type ServiceFeeRate: Get<u32>;
        /// Weight information
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    pub type PrepaidBalances<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

    #[pallet::storage]
    pub type ProxyConfigs<T: Config> = StorageDoubleMap<_, Blake2_128Concat, T::AccountId, Blake2_128Concat, T::AccountId, ProxyConfig<T::AccountId, BalanceOf<T>>, OptionQuery>;

    #[pallet::storage]
    pub type DepositAuthorizations<T: Config> = StorageDoubleMap<_, Blake2_128Concat, T::AccountId, Blake2_128Concat, T::AccountId, DepositAuthorization<BalanceOf<T>>, OptionQuery>;

    #[pallet::storage]
    pub type FeeRecords<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, BoundedVec<FeeRecord<BalanceOf<T>, BlockNumberFor<T>>, ConstU32<100>>, ValueQuery>;

    #[pallet::storage]
    pub type TotalServiceFees<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn pre_authorized_sponsors)]
    pub type PreAuthorizedSponsors<T: Config> = StorageMap<
        _, 
        Blake2_128Concat, 
        T::AccountId, 
        (BalanceOf<T>, T::BlockNumber, BalanceOf<T>, u32), // (额度上限, 过期高度, 每笔上限, 每块上限)
        OptionQuery
    >;

    #[pallet::storage]
    #[pallet::getter(fn system_pool)]
    pub type SystemPool<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn pending_auto_pays)]
    pub type PendingAutoPays<T: Config> = StorageValue<
        _, 
        BoundedVec<(T::AccountId, T::AccountId, BalanceOf<T>), ConstU32<10_000>>, // (赞助方, 受益人, 金额)
        ValueQuery
    >;

    #[pallet::storage]
    #[pallet::getter(fn sponsor_whitelist)]
    pub type SponsorWhitelist<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, (), OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn user_monthly_usage)]
    pub type UserMonthlyUsage<T: Config> = StorageDoubleMap<
        _, 
        Blake2_128Concat, T::AccountId, 
        Blake2_128Concat, u32, // 月份标识(yyyyMM)
        BalanceOf<T>,
        ValueQuery
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        PrepaidDeposited { user: T::AccountId, amount: BalanceOf<T> },
        PrepaidWithdrawn { user: T::AccountId, amount: BalanceOf<T> },
        ProxyAdded { user: T::AccountId, proxy: T::AccountId, permission: ProxyPermission },
        ProxyRemoved { user: T::AccountId, proxy: T::AccountId },
        ProxyExecuted { user: T::AccountId, proxy: T::AccountId, fee: BalanceOf<T> },
        BatchExecuted { user: T::AccountId, proxy: T::AccountId, count: u32, total_fee: BalanceOf<T> },
        ServiceFeeCollected { amount: BalanceOf<T> },
        /// 系统托管池资金增加 [增加金额, 新总额]
        SystemPoolIncreased { amount: BalanceOf<T>, new_total: BalanceOf<T> },
    }

    #[pallet::error]
    pub enum Error<T> {
        InsufficientBalance,
        ProxyNotFound,
        ProxyDisabled,
        InsufficientPermission,
        ExceedsFeeLimit,
        TooManyBatchCalls,
        BelowMinimumDeposit,
        ProxyAlreadyExists,
        Overflow,
        NotAuthorized,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// 用户充值预付费到paymaster账户
        #[pallet::weight(10_000)]
        pub fn deposit_prepaid(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(amount >= T::MinimumDeposit::get(), Error::<T>::BelowMinimumDeposit);
            // Transfer BUD from user to pallet account
            let pallet_account = T::PalletId::get().into_account_truncating();
            <T as Config>::Currency::transfer(&who, &pallet_account, amount, AllowDeath).map_err(|_| Error::<T>::Overflow)?;
            PrepaidBalances::<T>::mutate(&who, |b| *b = b.saturating_add(amount));
            Self::deposit_event(Event::PrepaidDeposited { user: who, amount });
            Ok(())
        }

        /// 第三方为用户充值预付费
        #[pallet::weight(10_000)]
        pub fn deposit_prepaid_for(origin: OriginFor<T>, beneficiary: T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
            let payer = ensure_signed(origin)?;
            ensure!(amount >= T::MinimumDeposit::get(), Error::<T>::BelowMinimumDeposit);
            // Transfer from payer to pallet
            let pallet_account = T::PalletId::get().into_account_truncating();
            <T as Config>::Currency::transfer(&payer, &pallet_account, amount, AllowDeath).map_err(|_| Error::<T>::Overflow)?;
            PrepaidBalances::<T>::mutate(&beneficiary, |b| *b = b.saturating_add(amount));
            Self::deposit_event(Event::PrepaidDeposited { user: beneficiary, amount });
            Ok(())
        }

        /// 使用预先授权进行充值
        #[pallet::weight(10_000)]
        pub fn authorized_deposit_prepaid(origin: OriginFor<T>, beneficiary: T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
            let payer = ensure_signed(origin)?;
            ensure!(amount >= T::MinimumDeposit::get(), Error::<T>::BelowMinimumDeposit);
            let auth = DepositAuthorizations::<T>::get(&beneficiary, &payer).ok_or(Error::<T>::NotAuthorized)?;
            ensure!(auth.enabled, Error::<T>::NotAuthorized);
            if let Some(max) = auth.max_amount { ensure!(amount <= max, Error::<T>::ExceedsFeeLimit); }
            let pallet_account = T::PalletId::get().into_account_truncating();
            <T as Config>::Currency::transfer(&payer, &pallet_account, amount, AllowDeath).map_err(|_| Error::<T>::Overflow)?;
            PrepaidBalances::<T>::mutate(&beneficiary, |b| *b = b.saturating_add(amount));
            Self::deposit_event(Event::PrepaidDeposited { user: beneficiary, amount });
            Ok(())
        }

        /// 设置充值授权
        #[pallet::weight(10_000)]
        pub fn set_deposit_authorization(origin: OriginFor<T>, authorized_payer: T::AccountId, max_amount: Option<BalanceOf<T>>, enabled: bool) -> DispatchResult {
            let who = ensure_signed(origin)?;
            DepositAuthorizations::<T>::insert(&who, &authorized_payer, DepositAuthorization { max_amount, enabled });
            Ok(())
        }

        /// 用户提取未使用的预付费余额
        #[pallet::weight(10_000)]
        pub fn withdraw_prepaid(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let pallet_account = T::PalletId::get().into_account_truncating();
            PrepaidBalances::<T>::try_mutate(&who, |b| -> DispatchResult {
                ensure!(*b >= amount, Error::<T>::InsufficientBalance);
                *b = b.saturating_sub(amount);
                Ok(())
            })?;
            <T as Config>::Currency::transfer(&pallet_account, &who, amount, AllowDeath).map_err(|_| Error::<T>::Overflow)?;
            Self::deposit_event(Event::PrepaidWithdrawn { user: who, amount });
            Ok(())
        }

        /// 为用户添加代理账户
        #[pallet::weight(10_000)]
        pub fn add_proxy(origin: OriginFor<T>, proxy: T::AccountId, permission: ProxyPermission, fee_limit: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(ProxyConfigs::<T>::get(&who, &proxy).is_none(), Error::<T>::ProxyAlreadyExists);
            let cfg = ProxyConfig { proxy: proxy.clone(), permission: permission.clone(), fee_limit, enabled: true };
            ProxyConfigs::<T>::insert(&who, &proxy, cfg);
            Self::deposit_event(Event::ProxyAdded { user: who, proxy, permission });
            Ok(())
        }

        /// 移除指定的代理账户
        #[pallet::weight(10_000)]
        pub fn remove_proxy(origin: OriginFor<T>, proxy: T::AccountId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(ProxyConfigs::<T>::take(&who, &proxy).is_some(), Error::<T>::ProxyNotFound);
            Self::deposit_event(Event::ProxyRemoved { user: who, proxy });
            Ok(())
        }

        /// 代理执行单笔交易（以预估费用扣除预付费，并收取服务费）
        #[pallet::weight(10_000)]
        pub fn proxy_execute(origin: OriginFor<T>, user: T::AccountId, call: Box<<T as Config>::RuntimeCall>, estimated_fee: BalanceOf<T>) -> DispatchResultWithPostInfo {
            let proxy = ensure_signed(origin)?;
            let cfg = ProxyConfigs::<T>::get(&user, &proxy).ok_or(Error::<T>::ProxyNotFound)?;
            ensure!(cfg.enabled, Error::<T>::ProxyDisabled);
            ensure!(estimated_fee <= cfg.fee_limit, Error::<T>::ExceedsFeeLimit);
            // 权限校验（示例：仅简单做，不做实际call解析）
            match cfg.permission { ProxyPermission::All | ProxyPermission::Custom(_) | ProxyPermission::Transfer | ProxyPermission::Governance => {} }

            // 预扣费：用户的预付费余额
            let mut total_fee = estimated_fee;
            let service_fee = estimated_fee.saturating_mul(T::ServiceFeeRate::get().into()) / 10_000u32.into();
            total_fee = total_fee.saturating_add(service_fee);

            PrepaidBalances::<T>::try_mutate(&user, |b| -> DispatchResult {
                ensure!(*b >= total_fee, Error::<T>::InsufficientBalance);
                *b = b.saturating_sub(total_fee);
                Ok(())
            })?;

            TotalServiceFees::<T>::mutate(|acc| *acc = acc.saturating_add(service_fee));

            // 实际执行调用：使用 dispatch_bypass_filter 代表由系统代付发起
            let origin = frame_system::RawOrigin::Signed(user.clone()).into();
            let info = call.dispatch(origin);

            match info {
                Ok(post) => {
                    // 记录
                    let mut records = FeeRecords::<T>::get(&user);
                    let record = FeeRecord { amount: estimated_fee, block_number: <frame_system::Pallet<T>>::block_number(), tx_hash: None };
                    let _ = records.try_push(record);
                    FeeRecords::<T>::insert(&user, records);
                    Self::deposit_event(Event::ProxyExecuted { user, proxy, fee: estimated_fee });
                    Ok(post)
                }
                Err(e) => {
                    // 失败：退款（不含服务费）
                    PrepaidBalances::<T>::mutate(&user, |b| *b = b.saturating_add(estimated_fee));
                    Err(e.error)
                }
            }
        }

        /// 代理批量执行多笔交易
        #[pallet::weight(10_000)]
        pub fn batch_proxy_execute(origin: OriginFor<T>, user: T::AccountId, calls: Vec<Box<<T as Config>::RuntimeCall>>, estimated_total_fee: BalanceOf<T>) -> DispatchResultWithPostInfo {
            let proxy = ensure_signed(origin)?;
            ensure!((calls.len() as u32) <= T::MaxBatchSize::get(), Error::<T>::TooManyBatchCalls);
            let cfg = ProxyConfigs::<T>::get(&user, &proxy).ok_or(Error::<T>::ProxyNotFound)?;
            ensure!(cfg.enabled, Error::<T>::ProxyDisabled);
            ensure!(estimated_total_fee <= cfg.fee_limit, Error::<T>::ExceedsFeeLimit);

            let service_fee = estimated_total_fee.saturating_mul(T::ServiceFeeRate::get().into()) / 10_000u32.into();
            let total_fee = estimated_total_fee.saturating_add(service_fee);

            PrepaidBalances::<T>::try_mutate(&user, |b| -> DispatchResult {
                ensure!(*b >= total_fee, Error::<T>::InsufficientBalance);
                *b = b.saturating_sub(total_fee);
                Ok(())
            })?;
            TotalServiceFees::<T>::mutate(|acc| *acc = acc.saturating_add(service_fee));

            let mut success = 0u32;
            for call in calls.into_iter() {
                let origin = frame_system::RawOrigin::Signed(user.clone()).into();
                if call.dispatch(origin).is_ok() { success = success.saturating_add(1); }
            }

            let mut records = FeeRecords::<T>::get(&user);
            let record = FeeRecord { amount: estimated_total_fee, block_number: <frame_system::Pallet<T>>::block_number(), tx_hash: None };
            let _ = records.try_push(record);
            FeeRecords::<T>::insert(&user, records);
            Self::deposit_event(Event::BatchExecuted { user, proxy, count: success, total_fee: estimated_total_fee });

            Ok(PostDispatchInfo { actual_weight: None, pays_fee: Pays::No })
        }
    }

    /// Paymaster Pallet 公开接口，供其他 pallet 调用
    impl<T: Config> Pallet<T> {
        /// 系统托管池资金增加接口 - 由其他 pallet 调用（如 exchange pallet）
        /// 此函数假定调用方已经处理好代币转账到 pallet 账户，只负责更新托管池记录
        pub fn increase_system_pool(amount: BalanceOf<T>) -> DispatchResult {
            SystemPool::<T>::mutate(|pool| {
                *pool = pool.saturating_add(amount);
                let new_total = *pool;
                Self::deposit_event(Event::SystemPoolIncreased { amount, new_total });
            });
            Ok(())
        }

        /// 获取 Paymaster 的 pallet 账户地址
        pub fn pallet_account() -> T::AccountId {
            T::PalletId::get().into_account_truncating()
        }

        /// 查询系统托管池余额
        pub fn get_system_pool_balance() -> BalanceOf<T> {
            SystemPool::<T>::get()
        }
    }

    pub trait WeightInfo { fn dummy() -> frame_support::weights::Weight { 0 } }
}