#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;
pub mod weights;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{pallet_prelude::*, BoundedVec};
    use pallet_escrow::pallet::Escrow as EscrowTrait;
    use frame_system::pallet_prelude::*;
    use sp_runtime::RuntimeDebug;
    use sp_runtime::traits::Saturating;

    // 占位类型，用于泛型 Balance；实际 Balance 在 Config::Balance
    // 这里不需要 BalanceOf 映射

    #[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, TypeInfo, MaxEncodedLen, RuntimeDebug)]
    pub enum Side { Buy, Sell }

    #[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
    pub struct Order<AccountId, Balance> { pub owner: AccountId, pub side: Side, pub price: Balance, pub amount: Balance, pub min: Balance, pub expiry: u32, pub active: bool }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type Balance: Parameter + Copy + Default + MaxEncodedLen + From<u128> + Into<u128>;
        type MaxNotesLen: Get<u32>;
        /// 托管接口：卖单挂单时将 BUD 锁入托管，撤单退款
        type Escrow: EscrowTrait<Self::AccountId, Self::Balance>;
        /// 权重信息
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    pub type Orders<T: Config> = StorageMap<_, Blake2_128Concat, u64, Order<T::AccountId, T::Balance>, OptionQuery>;
    #[pallet::storage]
    pub type NextOrderId<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> { OrderPlaced { id: u64 }, OrderCanceled { id: u64 } }

    #[pallet::error]
    pub enum Error<T> { NotFound, NotOwner }

    pub use crate::weights::WeightInfo;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// 挂单（最小骨架，仅登记，不做托管）
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::place_order())]
        pub fn place_order(origin: OriginFor<T>, side_code: u8, price: T::Balance, amount: T::Balance, min: T::Balance, expiry: u32) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let id = NextOrderId::<T>::mutate(|x| { let id=*x; *x=id.saturating_add(1); id });
            let side = match side_code { 0 => Side::Buy, 1 => Side::Sell, _ => Side::Buy };
            Orders::<T>::insert(id, Order { owner: who, side, price, amount, min, expiry, active: true });
            // 卖单：将 amount 锁入托管
            if matches!(side, Side::Sell) { T::Escrow::lock_from(&Orders::<T>::get(id).as_ref().unwrap().owner, id, amount)?; }
            Self::deposit_event(Event::OrderPlaced { id });
            Ok(())
        }
        /// 撤单
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::cancel_order())]
        pub fn cancel_order(origin: OriginFor<T>, id: u64) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let o = Orders::<T>::get(id).ok_or(Error::<T>::NotFound)?;
            ensure!(o.owner == who, Error::<T>::NotOwner);
            if matches!(o.side, Side::Sell) { T::Escrow::refund_all(id, &who)?; }
            Orders::<T>::remove(id);
            Self::deposit_event(Event::OrderCanceled { id });
            Ok(())
        }
    }
}


