#![cfg_attr(not(feature = "std"), no_std)]

// 函数级详细中文注释：
// 本 Pallet 管理“陵园/墓位”的生命周期、容量/占用与安葬/迁出记录。
// 支持将陵园管理员设置为多签账户（通过 pallet-multisig 复用），从而以多签方式执行关键操作。
// 与其他业务 Pallet（逝者/祭奠/订单）通过事件与可选回调进行低耦合联动。

extern crate alloc;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{pallet_prelude::*, BoundedVec};
    use frame_system::pallet_prelude::*;

    /// 函数级中文注释：墓位类型，Single=单人；Family(n)=可安葬 n 人。
    #[derive(Encode, Decode, Clone, Copy, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum PlotKind { Single, Family(u16) }

    /// 函数级中文注释：陵园结构，含管理员账户与元数据。
    #[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct Cemetery<BoundedCid, AccountId> {
        pub id: u64,
        pub admin: AccountId,
        pub meta_cid: Option<BoundedCid>,
        pub active: bool,
    }

    // 说明：Extrinsic 参数不再直接使用 PlotKind，而使用 u8 离散码映射，避免 DecodeWithMemTracking 约束。

    /// 函数级中文注释：墓位结构，记录容量/占用与归属。
    #[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct Plot<BoundedCid, AccountId> {
        pub id: u64,
        pub cemetery_id: u64,
        pub owner: AccountId,
        pub kind: PlotKind,
        pub capacity: u16,
        pub occupied: u16,
        pub meta_cid: Option<BoundedCid>,
        pub active: bool,
    }

    /// 函数级中文注释：安葬记录。
    #[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct IntermentRecord<BlockNumber> { pub plot_id: u64, pub deceased_id: u64, pub at: BlockNumber }

    /// 函数级中文注释：安葬/迁出后可选回调 Trait，默认 Noop。用于自动授权或订单自动验收。
    pub trait OnIntermentCommitted<AccountId> {
        fn on_interment(_plot_id: u64, _deceased_id: u64, _cemetery_admin: &AccountId) {}
        fn on_exhumation(_plot_id: u64, _deceased_id: u64, _cemetery_admin: &AccountId) {}
    }
    impl<AccountId> OnIntermentCommitted<AccountId> for () {}

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        #[pallet::constant] type MaxCidLen: Get<u32>;
        #[pallet::constant] type MaxPlotsPerCemetery: Get<u32>;
        #[pallet::constant] type MaxOccupantsPerPlot: Get<u32>;
        #[pallet::constant] type MaxMemoLen: Get<u32>;
        type OnIntermentCommitted: OnIntermentCommitted<Self::AccountId>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    type BoundedCidOf<T> = BoundedVec<u8, <T as Config>::MaxCidLen>;
    type BoundedMemoOf<T> = BoundedVec<u8, <T as Config>::MaxMemoLen>;

    #[pallet::storage]
    pub type Cemeteries<T: Config> = StorageMap<_, Blake2_128Concat, u64, Cemetery<BoundedCidOf<T>, T::AccountId>, OptionQuery>;
    #[pallet::storage]
    pub type Plots<T: Config> = StorageMap<_, Blake2_128Concat, u64, Plot<BoundedCidOf<T>, T::AccountId>, OptionQuery>;
    #[pallet::storage]
    pub type PlotsByCemetery<T: Config> = StorageMap<_, Blake2_128Concat, u64, BoundedVec<u64, T::MaxPlotsPerCemetery>, ValueQuery>;
    #[pallet::storage]
    pub type OccupantsOfPlot<T: Config> = StorageMap<_, Blake2_128Concat, u64, BoundedVec<u64, T::MaxOccupantsPerPlot>, ValueQuery>;
    #[pallet::storage]
    pub type PlotOfDeceased<T: Config> = StorageMap<_, Blake2_128Concat, u64, u64, OptionQuery>;
    #[pallet::storage]
    pub type Interments<T: Config> = StorageMap<_, Blake2_128Concat, (u64, u64), IntermentRecord<BlockNumberFor<T>>, OptionQuery>;
    #[pallet::storage]
    pub type NextCemeteryId<T: Config> = StorageValue<_, u64, ValueQuery>;
    #[pallet::storage]
    pub type NextPlotId<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        CemeteryCreated { id: u64 },
        CemeteryAdminChanged { id: u64 },
        PlotCreated { id: u64, cemetery_id: u64 },
        PlotUpdated { id: u64 },
        PlotTransferred { id: u64, new_owner: T::AccountId },
        IntermentCommitted { plot_id: u64, deceased_id: u64 },
        ExhumationCommitted { plot_id: u64, deceased_id: u64 },
    }

    #[pallet::error]
    pub enum Error<T> {
        CemeteryNotFound,
        PlotNotFound,
        PlotInactive,
        PlotFull,
        AlreadyInterred,
        NotAuthorized,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// 函数级中文注释：创建陵园，设置管理员（可为多签账户），并可附元数据 CID（建议链下加密）。
        #[pallet::weight(10_000)]
        pub fn create_cemetery(origin: OriginFor<T>, admin: Option<T::AccountId>, meta_cid: Option<BoundedCidOf<T>>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let id = NextCemeteryId::<T>::mutate(|n| { let id = *n; *n = n.saturating_add(1); id });
            let cem = Cemetery { id, admin: admin.unwrap_or(who), meta_cid, active: true };
            ensure!(Cemeteries::<T>::get(id).is_none(), Error::<T>::CemeteryNotFound);
            Cemeteries::<T>::insert(id, cem);
            Self::deposit_event(Event::<T>::CemeteryCreated { id });
            Ok(())
        }

        /// 函数级中文注释：切换陵园管理员（可切到多签账户）。仅现管理员可调用。
        #[pallet::weight(10_000)]
        pub fn set_cemetery_admin(origin: OriginFor<T>, cemetery_id: u64, new_admin: T::AccountId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Cemeteries::<T>::try_mutate(cemetery_id, |maybe| -> DispatchResult {
                let c = maybe.as_mut().ok_or(Error::<T>::CemeteryNotFound)?;
                ensure!(c.admin == who, Error::<T>::NotAuthorized);
                c.admin = new_admin;
                Ok(())
            })?;
            Self::deposit_event(Event::<T>::CemeteryAdminChanged { id: cemetery_id });
            Ok(())
        }

        /// 函数级中文注释：创建墓位。容量对 Single 默认 1，对 Family(n) 为 n。owner 默认为调用者。
        #[pallet::weight(10_000)]
        pub fn create_plot(origin: OriginFor<T>, cemetery_id: u64, kind: u8, capacity: Option<u16>, owner: Option<T::AccountId>, meta_cid: Option<BoundedCidOf<T>>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let cem = Cemeteries::<T>::get(cemetery_id).ok_or(Error::<T>::CemeteryNotFound)?;
            ensure!(cem.admin == who, Error::<T>::NotAuthorized);

            let id = NextPlotId::<T>::mutate(|n| { let id = *n; *n = n.saturating_add(1); id });
            let kind_enum = match kind { 0 => PlotKind::Single, _ => PlotKind::Family(1) };
            let cap = match kind_enum { PlotKind::Single => 1, PlotKind::Family(n) => n };
            let plot = Plot { id, cemetery_id, owner: owner.unwrap_or(who), kind: kind_enum, capacity: capacity.unwrap_or(cap), occupied: 0, meta_cid, active: true };
            Plots::<T>::insert(id, &plot);
            PlotsByCemetery::<T>::mutate(cemetery_id, |v| { let _ = v.try_push(id); });
            Self::deposit_event(Event::<T>::PlotCreated { id, cemetery_id });
            Ok(())
        }

        /// 函数级中文注释：更新墓位元数据/上下架。仅陵园管理员可操作。
        #[pallet::weight(10_000)]
        pub fn update_plot(origin: OriginFor<T>, plot_id: u64, meta_cid: Option<Option<BoundedCidOf<T>>>, active: Option<bool>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let cemetery_id = Plots::<T>::try_mutate(plot_id, |maybe| -> Result<u64, DispatchError> {
                let p = maybe.as_mut().ok_or(Error::<T>::PlotNotFound)?;
                if let Some(cid_opt) = meta_cid { p.meta_cid = cid_opt; }
                if let Some(a) = active { p.active = a; }
                Ok(p.cemetery_id)
            })?;
            let cem = Cemeteries::<T>::get(cemetery_id).ok_or(Error::<T>::CemeteryNotFound)?;
            ensure!(cem.admin == who, Error::<T>::NotAuthorized);
            Self::deposit_event(Event::<T>::PlotUpdated { id: plot_id });
            Ok(())
        }

        /// 函数级中文注释：转让墓位所有权。仅陵园管理员可操作。
        #[pallet::weight(10_000)]
        pub fn transfer_plot(origin: OriginFor<T>, plot_id: u64, new_owner: T::AccountId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let cemetery_id = Plots::<T>::try_mutate(plot_id, |maybe| -> Result<u64, DispatchError> {
                let p = maybe.as_mut().ok_or(Error::<T>::PlotNotFound)?;
                p.owner = new_owner.clone();
                Ok(p.cemetery_id)
            })?;
            let cem = Cemeteries::<T>::get(cemetery_id).ok_or(Error::<T>::CemeteryNotFound)?;
            ensure!(cem.admin == who, Error::<T>::NotAuthorized);
            Self::deposit_event(Event::<T>::PlotTransferred { id: plot_id, new_owner });
            Ok(())
        }

        /// 函数级中文注释：安葬操作（添加逝者到墓位）。仅陵园管理员可调用。
        #[pallet::weight(10_000)]
        pub fn inter(origin: OriginFor<T>, plot_id: u64, deceased_id: u64, _memo: Option<BoundedMemoOf<T>>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let cemetery_id = {
                let p = Plots::<T>::get(plot_id).ok_or(Error::<T>::PlotNotFound)?;
                ensure!(p.active, Error::<T>::PlotInactive);
                ensure!(p.occupied < p.capacity, Error::<T>::PlotFull);
                p.cemetery_id
            };
            let cem = Cemeteries::<T>::get(cemetery_id).ok_or(Error::<T>::CemeteryNotFound)?;
            ensure!(cem.admin == who, Error::<T>::NotAuthorized);

            ensure!(Interments::<T>::get((plot_id, deceased_id)).is_none(), Error::<T>::AlreadyInterred);
            let now = <frame_system::Pallet<T>>::block_number();
            Interments::<T>::insert((plot_id, deceased_id), IntermentRecord { plot_id, deceased_id, at: now });
            OccupantsOfPlot::<T>::mutate(plot_id, |v| { let _ = v.try_push(deceased_id); });
            Plots::<T>::mutate(plot_id, |maybe| { if let Some(p) = maybe { p.occupied = p.occupied.saturating_add(1); }});

            Self::deposit_event(Event::<T>::IntermentCommitted { plot_id, deceased_id });
            <T as Config>::OnIntermentCommitted::on_interment(plot_id, deceased_id, &cem.admin);
            Ok(())
        }

        /// 函数级中文注释：迁出（起掘）。仅陵园管理员可调用。
        #[pallet::weight(10_000)]
        pub fn exhume(origin: OriginFor<T>, plot_id: u64, deceased_id: u64) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let cemetery_id = {
                let p = Plots::<T>::get(plot_id).ok_or(Error::<T>::PlotNotFound)?;
                p.cemetery_id
            };
            let cem = Cemeteries::<T>::get(cemetery_id).ok_or(Error::<T>::CemeteryNotFound)?;
            ensure!(cem.admin == who, Error::<T>::NotAuthorized);

            if Interments::<T>::take((plot_id, deceased_id)).is_some() {
                OccupantsOfPlot::<T>::mutate(plot_id, |v| {
                    if let Some(pos) = v.iter().position(|x| *x == deceased_id) { v.swap_remove(pos); }
                });
                Plots::<T>::mutate(plot_id, |maybe| { if let Some(p) = maybe { p.occupied = p.occupied.saturating_sub(1); }});
                Self::deposit_event(Event::<T>::ExhumationCommitted { plot_id, deceased_id });
                <T as Config>::OnIntermentCommitted::on_exhumation(plot_id, deceased_id, &cem.admin);
            }
            Ok(())
        }
    }
}


