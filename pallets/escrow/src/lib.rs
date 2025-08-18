#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{pallet_prelude::*, traits::{Currency, ExistenceRequirement}, PalletId};
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{AccountIdConversion, Saturating, Zero};

    pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// 供其他 Pallet 内部调用的托管接口
    pub trait Escrow<AccountId, Balance> {
        /// 从付款人转入托管并记录
        fn lock_from(payer: &AccountId, id: u64, amount: Balance) -> DispatchResult;
        /// 从托管转出部分金额到指定账户（可多次分账），直至全部转出
        fn transfer_from_escrow(id: u64, to: &AccountId, amount: Balance) -> DispatchResult;
        /// 将托管全部释放给收款人
        fn release_all(id: u64, to: &AccountId) -> DispatchResult;
        /// 将托管全部退款给收款人
        fn refund_all(id: u64, to: &AccountId) -> DispatchResult;
        /// 查询当前托管余额
        fn amount_of(id: u64) -> Balance;
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type Currency: Currency<Self::AccountId>;
        type EscrowPalletId: Get<PalletId>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// 简单托管：订单 -> 锁定余额
    #[pallet::storage]
    pub type Locked<T: Config> = StorageMap<_, Blake2_128Concat, u64, BalanceOf<T>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> { Locked { id: u64, amount: BalanceOf<T> }, Released { id: u64, to: T::AccountId, amount: BalanceOf<T> }, Refunded { id: u64, to: T::AccountId, amount: BalanceOf<T> } }

    #[pallet::error]
    pub enum Error<T> { Insufficient, NoLock }

    impl<T: Config> Pallet<T> {
        fn account() -> T::AccountId { T::EscrowPalletId::get().into_account_truncating() }
    }

    impl<T: Config> Escrow<T::AccountId, BalanceOf<T>> for Pallet<T> {
        fn lock_from(payer: &T::AccountId, id: u64, amount: BalanceOf<T>) -> DispatchResult {
            let escrow = Self::account();
            T::Currency::transfer(payer, &escrow, amount, ExistenceRequirement::KeepAlive).map_err(|_| Error::<T>::Insufficient)?;
            let cur = Locked::<T>::get(id);
            Locked::<T>::insert(id, cur.saturating_add(amount));
            Self::deposit_event(Event::Locked { id, amount });
            Ok(())
        }
        fn transfer_from_escrow(id: u64, to: &T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
            let cur = Locked::<T>::get(id);
            ensure!(!cur.is_zero(), Error::<T>::NoLock);
            let new = cur.saturating_sub(amount);
            Locked::<T>::insert(id, new);
            let escrow = Self::account();
            T::Currency::transfer(&escrow, to, amount, ExistenceRequirement::KeepAlive).map_err(|_| Error::<T>::NoLock)?;
            if new.is_zero() { Locked::<T>::remove(id); }
            Ok(())
        }
        fn release_all(id: u64, to: &T::AccountId) -> DispatchResult {
            let amount = Locked::<T>::take(id);
            let escrow = Self::account();
            T::Currency::transfer(&escrow, to, amount, ExistenceRequirement::KeepAlive).map_err(|_| Error::<T>::NoLock)?;
            Self::deposit_event(Event::Released { id, to: to.clone(), amount });
            Ok(())
        }
        fn refund_all(id: u64, to: &T::AccountId) -> DispatchResult {
            let amount = Locked::<T>::take(id);
            let escrow = Self::account();
            T::Currency::transfer(&escrow, to, amount, ExistenceRequirement::KeepAlive).map_err(|_| Error::<T>::NoLock)?;
            Self::deposit_event(Event::Refunded { id, to: to.clone(), amount });
            Ok(())
        }
        fn amount_of(id: u64) -> BalanceOf<T> { Locked::<T>::get(id) }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// 锁定：从付款人划转到托管账户并记录
        #[pallet::call_index(0)]
        #[pallet::weight(10_000)]
        pub fn lock(origin: OriginFor<T>, id: u64, payer: T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
            let _ = ensure_signed(origin)?; // 可限制为市场/订单 pallet 调用
            <Self as Escrow<T::AccountId, BalanceOf<T>>>::lock_from(&payer, id, amount)
        }
        /// 释放：将托管金额转给收款人
        #[pallet::call_index(1)]
        #[pallet::weight(10_000)]
        pub fn release(origin: OriginFor<T>, id: u64, to: T::AccountId) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            <Self as Escrow<T::AccountId, BalanceOf<T>>>::release_all(id, &to)
        }
        /// 退款：退回付款人
        #[pallet::call_index(2)]
        #[pallet::weight(10_000)]
        pub fn refund(origin: OriginFor<T>, id: u64, to: T::AccountId) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            <Self as Escrow<T::AccountId, BalanceOf<T>>>::refund_all(id, &to)
        }
    }
}


