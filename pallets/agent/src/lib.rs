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

    // ====== 定价能力扩展（仅最小骨架，后续可细化） ======
    /// 函数级中文注释：价格条目。version 用于在订单创建时固化价格快照。
    #[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
    pub struct PriceEntry<Balance> { pub version: u32, pub amount: Balance, pub active: bool }

    /// 函数级中文注释：Agent 对某 ritual 规格的可执行能力与当前价格。
    #[pallet::storage]
    pub type AgentOfferings<T: Config> = StorageDoubleMap<_, Blake2_128Concat, T::AccountId, Blake2_128Concat, u64, bool, ValueQuery>;
    #[pallet::storage]
    pub type AgentPrice<T: Config> = StorageDoubleMap<_, Blake2_128Concat, T::AccountId, Blake2_128Concat, u64, PriceEntry<BalanceOf<T>>, OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        AgentRegistered { who: T::AccountId },
        /// 函数级中文注释：登记/撤销可执行某 ritual 规格能力。
        OfferingToggled { who: T::AccountId, spec_id: u64, active: bool },
        /// 函数级中文注释：设置价格（版本自增或初始化）。
        PriceSet { who: T::AccountId, spec_id: u64, version: u32, amount: BalanceOf<T>, active: bool },
    }

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

        /// 函数级中文注释：登记可执行某 ritual 规格能力；active=true/false 进行开关。
        #[pallet::call_index(1)]
        #[pallet::weight(10_000)]
        pub fn toggle_offering(origin: OriginFor<T>, spec_id: u64, active: bool) -> DispatchResult {
            let who = ensure_signed(origin)?;
            AgentOfferings::<T>::insert(&who, spec_id, active);
            Self::deposit_event(Event::<T>::OfferingToggled { who, spec_id, active });
            Ok(())
        }

        /// 函数级中文注释：设置/更新价格。若不存在则 version=1；存在则 version+=1。
        #[pallet::call_index(2)]
        #[pallet::weight(10_000)]
        pub fn set_price(origin: OriginFor<T>, spec_id: u64, amount: BalanceOf<T>, active: bool) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let next_ver = match AgentPrice::<T>::get(&who, spec_id) { Some(p) => p.version.saturating_add(1), None => 1 };
            AgentPrice::<T>::insert(&who, spec_id, PriceEntry { version: next_ver, amount, active });
            Self::deposit_event(Event::<T>::PriceSet { who, spec_id, version: next_ver, amount, active });
            Ok(())
        }
    }
}


