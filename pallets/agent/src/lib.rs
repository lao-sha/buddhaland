#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ReservableCurrency},
        BoundedVec,
    };
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type Currency: ReservableCurrency<Self::AccountId>;
        type MaxSkills: Get<u32>;
        type MaxCalendar: Get<u32>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(MaxSkills))]
    pub struct Agent<Balance, MaxSkills: Get<u32>> {
        pub stake: Balance,
        pub skills: BoundedVec<u8, MaxSkills>,
        pub region_hash: sp_core::H256,
        pub active: bool,
        pub rating_sum: u32,
        pub rating_cnt: u32,
    }

    #[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
    pub struct Slot<BlockNumber> { pub date_block: BlockNumber }

    pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::storage]
    pub type Agents<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, Agent<BalanceOf<T>, T::MaxSkills>, OptionQuery>;
    #[pallet::storage]
    pub type Availabilities<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, BoundedVec<Slot<BlockNumberFor<T>>, T::MaxCalendar>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> { AgentRegistered { who: T::AccountId } }

    #[pallet::error]
    pub enum Error<T> { AlreadyRegistered }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// 注册代办人：保留一定质押，登记技能与区域
        #[pallet::call_index(0)]
        #[pallet::weight(10_000)]
        pub fn register_agent(origin: OriginFor<T>, stake: BalanceOf<T>, skills: BoundedVec<u8, T::MaxSkills>, region_hash: sp_core::H256) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Agents::<T>::get(&who).is_none(), Error::<T>::AlreadyRegistered);
            T::Currency::reserve(&who, stake).map_err(|_| Error::<T>::AlreadyRegistered)?;
            Agents::<T>::insert(&who, Agent { stake, skills, region_hash, active: true, rating_sum: 0, rating_cnt: 0 });
            Self::deposit_event(Event::AgentRegistered { who });
            Ok(())
        }
    }
}


