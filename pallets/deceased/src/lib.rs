#![cfg_attr(not(feature = "std"), no_std)]

// 函数级详细中文注释：
// 本 Pallet 管理“逝者档案”与“族谱关系”（父母/配偶）。
// 敏感资料不上链明文，仅存承诺哈希与密文 CID；
// 提供 owner + editors 权限模型，便于陵园多签或家属共同维护档案。

extern crate alloc;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{pallet_prelude::*, BoundedVec};
    use frame_system::pallet_prelude::*;

    /// 函数级中文注释：关系类型。Parent/Spouse 两类，Child 可由 Parent 反向推导。
    #[derive(Encode, Decode, Clone, Copy, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum RelationKind { Parent, Spouse }

    /// 函数级中文注释：逝者档案。
    #[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct DeceasedRecord<AccountId, BoundedCid> {
        pub id: u64,
        pub owner: AccountId,
        pub profile_commit: sp_core::H256,
        pub meta_ver: u8,
        pub meta_cid: Option<BoundedCid>,
        pub active: bool,
    }

    // 说明：Extrinsic 参数不再直接使用 RelationKind，而使用 u8 离散码映射，避免 DecodeWithMemTracking 约束。

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        #[pallet::constant] type MaxCidLen: Get<u32>;
        #[pallet::constant] type MaxEditors: Get<u32>;
        #[pallet::constant] type MaxRelationsPerNode: Get<u32>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    type BoundedCidOf<T> = BoundedVec<u8, <T as Config>::MaxCidLen>;

    #[pallet::storage]
    pub type Deceaseds<T: Config> = StorageMap<_, Blake2_128Concat, u64, DeceasedRecord<T::AccountId, BoundedCidOf<T>>, OptionQuery>;
    #[pallet::storage]
    pub type NextDeceasedId<T: Config> = StorageValue<_, u64, ValueQuery>;
    #[pallet::storage]
    pub type DeceasedEditors<T: Config> = StorageMap<_, Blake2_128Concat, u64, BoundedVec<T::AccountId, T::MaxEditors>, ValueQuery>;
    #[pallet::storage]
    pub type Relations<T: Config> = StorageDoubleMap<_, Blake2_128Concat, u64, Blake2_128Concat, RelationKind, BoundedVec<u64, T::MaxRelationsPerNode>, ValueQuery>;
    #[pallet::storage]
    pub type InverseRelations<T: Config> = StorageDoubleMap<_, Blake2_128Concat, u64, Blake2_128Concat, RelationKind, BoundedVec<u64, T::MaxRelationsPerNode>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        DeceasedRegistered { id: u64 },
        DeceasedUpdated { id: u64 },
        DeceasedDeactivated { id: u64 },
        DeceasedOwnershipTransferred { id: u64 },
        EditorGranted { id: u64 },
        EditorRevoked { id: u64 },
        // 函数级中文注释：关系事件中 kind 使用离散码（0=Parent,1=Spouse）以避免 DecodeWithMemTracking 约束。
        RelationLinked { a: u64, b: u64, kind: u8 },
        RelationUnlinked { a: u64, b: u64, kind: u8 },
    }

    #[pallet::error]
    pub enum Error<T> {
        NotFound,
        NotAuthorized,
        AlreadyExists,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// 函数级中文注释：注册逝者档案。owner=origin，敏感资料仅存承诺与 CID。
        #[pallet::weight(10_000)]
        pub fn register_deceased(origin: OriginFor<T>, profile_commit: sp_core::H256, meta_ver: u8, meta_cid: Option<BoundedCidOf<T>>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let id = NextDeceasedId::<T>::mutate(|n| { let id = *n; *n = n.saturating_add(1); id });
            let rec = DeceasedRecord { id, owner: who.clone(), profile_commit, meta_ver, meta_cid, active: true };
            ensure!(Deceaseds::<T>::get(id).is_none(), Error::<T>::AlreadyExists);
            Deceaseds::<T>::insert(id, rec);
            Self::deposit_event(Event::<T>::DeceasedRegistered { id });
            Ok(())
        }

        /// 函数级中文注释：更新承诺/版本/CID。权限：owner 或 editors。
        #[pallet::weight(10_000)]
        pub fn update_deceased(origin: OriginFor<T>, deceased_id: u64, new_profile_commit: Option<sp_core::H256>, new_meta_ver: Option<u8>, new_meta_cid: Option<Option<BoundedCidOf<T>>>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Deceaseds::<T>::try_mutate(deceased_id, |maybe| -> DispatchResult {
                let rec = maybe.as_mut().ok_or(Error::<T>::NotFound)?;
                ensure!(rec.owner == who || DeceasedEditors::<T>::get(deceased_id).contains(&who), Error::<T>::NotAuthorized);
                if let Some(c) = new_profile_commit { rec.profile_commit = c; }
                if let Some(v) = new_meta_ver { rec.meta_ver = v; }
                if let Some(cid_opt) = new_meta_cid { rec.meta_cid = cid_opt; }
                Ok(())
            })?;
            Self::deposit_event(Event::<T>::DeceasedUpdated { id: deceased_id });
            Ok(())
        }

        /// 函数级中文注释：软删除/停用档案。权限：owner 或 editors。
        #[pallet::weight(10_000)]
        pub fn deactivate_deceased(origin: OriginFor<T>, deceased_id: u64) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Deceaseds::<T>::try_mutate(deceased_id, |maybe| -> DispatchResult {
                let rec = maybe.as_mut().ok_or(Error::<T>::NotFound)?;
                ensure!(rec.owner == who || DeceasedEditors::<T>::get(deceased_id).contains(&who), Error::<T>::NotAuthorized);
                rec.active = false;
                Ok(())
            })?;
            Self::deposit_event(Event::<T>::DeceasedDeactivated { id: deceased_id });
            Ok(())
        }

        /// 函数级中文注释：转移档案所有权（例如从陵园多签转给家属）。
        #[pallet::weight(10_000)]
        pub fn transfer_deceased_ownership(origin: OriginFor<T>, deceased_id: u64, new_owner: T::AccountId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Deceaseds::<T>::try_mutate(deceased_id, |maybe| -> DispatchResult {
                let rec = maybe.as_mut().ok_or(Error::<T>::NotFound)?;
                ensure!(rec.owner == who, Error::<T>::NotAuthorized);
                rec.owner = new_owner;
                Ok(())
            })?;
            Self::deposit_event(Event::<T>::DeceasedOwnershipTransferred { id: deceased_id });
            Ok(())
        }

        /// 函数级中文注释：授予/撤销编辑者（可把陵园多签加入为 editor）。
        #[pallet::weight(10_000)]
        pub fn grant_deceased_editor(origin: OriginFor<T>, deceased_id: u64, editor: T::AccountId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let owner = Deceaseds::<T>::get(deceased_id).ok_or(Error::<T>::NotFound)?.owner;
            ensure!(owner == who, Error::<T>::NotAuthorized);
            DeceasedEditors::<T>::mutate(deceased_id, |v| { let _ = v.try_push(editor.clone()); });
            Self::deposit_event(Event::<T>::EditorGranted { id: deceased_id });
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn revoke_deceased_editor(origin: OriginFor<T>, deceased_id: u64, editor: T::AccountId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let owner = Deceaseds::<T>::get(deceased_id).ok_or(Error::<T>::NotFound)?.owner;
            ensure!(owner == who, Error::<T>::NotAuthorized);
            DeceasedEditors::<T>::mutate(deceased_id, |v| { if let Some(pos) = v.iter().position(|x| *x == editor) { v.swap_remove(pos); }});
            Self::deposit_event(Event::<T>::EditorRevoked { id: deceased_id });
            Ok(())
        }

        /// 函数级中文注释：建立/解除族谱关系；去重与自环校验简化为最小实现。
        #[pallet::weight(10_000)]
        pub fn link_relation(origin: OriginFor<T>, a: u64, b: u64, kind: u8) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            ensure!(a != b, Error::<T>::AlreadyExists);
            let kind_enum = if kind == 0 { RelationKind::Parent } else { RelationKind::Spouse };
            Relations::<T>::mutate(a, kind_enum, |v| { if !v.contains(&b) { let _ = v.try_push(b); }});
            // Spouse 对称；Parent 反向统计到 InverseRelations(Parent) 即可供查询
            InverseRelations::<T>::mutate(b, kind_enum, |v| { if !v.contains(&a) { let _ = v.try_push(a); }});
            Self::deposit_event(Event::<T>::RelationLinked { a, b, kind });
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn unlink_relation(origin: OriginFor<T>, a: u64, b: u64, kind: u8) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            let kind_enum = if kind == 0 { RelationKind::Parent } else { RelationKind::Spouse };
            Relations::<T>::mutate(a, kind_enum, |v| { if let Some(pos) = v.iter().position(|x| *x == b) { v.swap_remove(pos); }});
            InverseRelations::<T>::mutate(b, kind_enum, |v| { if let Some(pos) = v.iter().position(|x| *x == a) { v.swap_remove(pos); }});
            Self::deposit_event(Event::<T>::RelationUnlinked { a, b, kind });
            Ok(())
        }
    }
}


