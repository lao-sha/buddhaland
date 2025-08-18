#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        BoundedVec,
    };
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_runtime::RuntimeDebug;

    /// 服务类型（供灯/供花/供果/供香/放生/供僧/建寺/添油/印经 等）
    #[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum ServiceKind { Light, Flower, Fruit, Incense, Release, Monk, Build, Oil, Sutra }

    /// 寺庙基础信息（仅存哈希与轻量字段）
    #[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
    pub struct Temple<AccountId> {
        pub owner: AccountId,
        pub name_hash: H256,
        pub geo_hash: H256,
        pub profile_hash: H256,
        pub active: bool,
    }

    /// 服务条目
    #[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(MaxTiers))]
    pub struct Service<Balance, MaxTiers: Get<u32>> {
        pub kind: ServiceKind,
        pub title_hash: H256,
        pub desc_hash: H256,
        pub price_tiers: BoundedVec<Balance, MaxTiers>,
        pub min_custom: Balance,
        pub max_custom: Balance,
        pub active: bool,
    }

    /// 日历槽位（公历日 + 标注）
    #[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
    pub struct CalendarSlot<BlockNumber> { pub date_block: BlockNumber, pub lunar_tag: u8 }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type MaxPriceTiers: Get<u32>;
        type MaxCalendar: Get<u32>;
        /// 价格/金额所用的链上余额类型（与 runtime Balance 对齐）
        type Balance: Parameter + MaxEncodedLen + Default + Copy;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    pub type TempleId = u32;
    pub type ServiceId = u32;

    #[pallet::storage]
    pub type Temples<T: Config> = StorageMap<_, Blake2_128Concat, TempleId, Temple<T::AccountId>, OptionQuery>;
    #[pallet::storage]
    pub type Services<T: Config> = StorageDoubleMap<
        _, Blake2_128Concat, TempleId, Blake2_128Concat, ServiceId,
        Service<BalanceOf<T>, T::MaxPriceTiers>, OptionQuery
    >;
    #[pallet::storage]
    pub type Calendars<T: Config> = StorageDoubleMap<
        _, Blake2_128Concat, (TempleId, ServiceId), Blake2_128Concat, u32,
        BoundedVec<CalendarSlot<BlockNumberFor<T>>, T::MaxCalendar>, OptionQuery
    >;
    #[pallet::storage]
    pub type NextTempleId<T: Config> = StorageValue<_, TempleId, ValueQuery>;
    #[pallet::storage]
    pub type NextServiceId<T: Config> = StorageMap<_, Blake2_128Concat, TempleId, ServiceId, ValueQuery>;

    /// 金额类型别名
    pub type BalanceOf<T> = <T as Config>::Balance;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        TempleRegistered { id: TempleId },
        ServiceAdded { temple: TempleId, service: ServiceId },
        CalendarUpdated { temple: TempleId, service: ServiceId },
    }

    #[pallet::error]
    pub enum Error<T> { TempleNotFound, ServiceNotFound, NotOwner, InvalidKind }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// 注册寺庙（仅存哈希）
        #[pallet::call_index(0)]
        #[pallet::weight(10_000)]
        pub fn register_temple(origin: OriginFor<T>, name_hash: H256, geo_hash: H256, profile_hash: H256) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let id = NextTempleId::<T>::mutate(|x| { let id=*x; *x=id.saturating_add(1); id });
            Temples::<T>::insert(id, Temple { owner: who, name_hash, geo_hash, profile_hash, active: true });
            Self::deposit_event(Event::TempleRegistered { id });
            Ok(())
        }
        /// 添加服务（寺庙所有者）
        #[pallet::call_index(1)]
        /// 函数级中文注释：
        /// 添加服务（寺庙所有者）。为规避 Extrinsic 参数上的自定义枚举解码限制，这里使用 `kind_code: u8`，在内部映射到 `ServiceKind`。
        /// 价格分级与自定义金额使用运行时配置的 `Balance` 类型，避免跨类型转换。
        #[pallet::weight(10_000)]
        pub fn add_service(origin: OriginFor<T>, temple: TempleId, kind_code: u8, title_hash: H256, desc_hash: H256, price_tiers: BoundedVec<BalanceOf<T>, T::MaxPriceTiers>, min_custom: BalanceOf<T>, max_custom: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let tp = Temples::<T>::get(temple).ok_or(Error::<T>::TempleNotFound)?;
            ensure!(tp.owner == who, Error::<T>::NotOwner);
            let sid = NextServiceId::<T>::mutate(temple, |x| { let id=*x; *x=id.saturating_add(1); id });
            let kind = match kind_code {
                0 => ServiceKind::Light,
                1 => ServiceKind::Flower,
                2 => ServiceKind::Fruit,
                3 => ServiceKind::Incense,
                4 => ServiceKind::Release,
                5 => ServiceKind::Monk,
                6 => ServiceKind::Build,
                7 => ServiceKind::Oil,
                8 => ServiceKind::Sutra,
                _ => return Err(Error::<T>::InvalidKind.into()),
            };
            Services::<T>::insert(temple, sid, Service { kind, title_hash, desc_hash, price_tiers, min_custom, max_custom, active: true });
            Self::deposit_event(Event::ServiceAdded { temple, service: sid });
            Ok(())
        }
    }
}


