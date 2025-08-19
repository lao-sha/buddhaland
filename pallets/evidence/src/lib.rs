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
	use alloc::vec::Vec;

	/// 函数级中文注释：共享证据（媒体）记录结构，存储跨域使用的图片/视频/文档的 IPFS CID（或哈希）。
	#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(MaxCidLen, MaxImg, MaxVid, MaxDoc, MaxMemoLen))]
	pub struct Evidence<
		AccountId,
		MaxCidLen: Get<u32>,
		MaxImg: Get<u32>,
		MaxVid: Get<u32>,
		MaxDoc: Get<u32>,
		MaxMemoLen: Get<u32>,
	> {
		pub id: u64,
		pub domain: u8,
		pub target_id: u64,
		pub owner: AccountId,
		pub imgs: BoundedVec<BoundedVec<u8, MaxCidLen>, MaxImg>,
		pub vids: BoundedVec<BoundedVec<u8, MaxCidLen>, MaxVid>,
		pub docs: BoundedVec<BoundedVec<u8, MaxCidLen>, MaxDoc>,
		pub memo: Option<BoundedVec<u8, MaxMemoLen>>,
	}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		#[pallet::constant] type MaxCidLen: Get<u32>;
		#[pallet::constant] type MaxImg: Get<u32>;
		#[pallet::constant] type MaxVid: Get<u32>;
		#[pallet::constant] type MaxDoc: Get<u32>;
		#[pallet::constant] type MaxMemoLen: Get<u32>;
		/// 通过 Authorizer 控制谁可提交/链接证据
		#[pallet::constant] type EvidenceNsBytes: Get<[u8; 8]>;
		/// 授权适配器：由 runtime 桥接到 pallet-authorizer，避免本 Pallet 直接依赖其 Config 约束
		type Authorizer: EvidenceAuthorizer<Self::AccountId>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	pub type NextEvidenceId<T: Config> = StorageValue<_, u64, ValueQuery>;
	#[pallet::storage]
	pub type Evidences<T: Config> = StorageMap<_, Blake2_128Concat, u64, Evidence<
		T::AccountId, T::MaxCidLen, T::MaxImg, T::MaxVid, T::MaxDoc, T::MaxMemoLen
	>, OptionQuery>;
	#[pallet::storage]
	pub type EvidenceByTarget<T: Config> = StorageDoubleMap<_, Blake2_128Concat, (u8, u64), Blake2_128Concat, u64, (), OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		EvidenceCommitted { id: u64, domain: u8, target_id: u64, owner: T::AccountId },
		EvidenceLinked { domain: u8, target_id: u64, id: u64 },
		EvidenceUnlinked { domain: u8, target_id: u64, id: u64 },
	}

	#[pallet::error]
	pub enum Error<T> { NotAuthorized, NotFound }

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// 函数级中文注释：提交证据，生成 EvidenceId 并落库；仅授权账户可提交。
		#[pallet::weight(10_000)]
		pub fn commit(
			origin: OriginFor<T>,
			domain: u8,
			target_id: u64,
			imgs: Vec<BoundedVec<u8, T::MaxCidLen>>,
			vids: Vec<BoundedVec<u8, T::MaxCidLen>>,
			docs: Vec<BoundedVec<u8, T::MaxCidLen>>,
			memo: Option<BoundedVec<u8, T::MaxMemoLen>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			// Authorizer 鉴权（通过适配器，解耦到 runtime）
			let ns = T::EvidenceNsBytes::get();
			ensure!(<T as Config>::Authorizer::is_authorized(ns, &who), Error::<T>::NotAuthorized);
			let id = NextEvidenceId::<T>::mutate(|n| { let id = *n; *n = n.saturating_add(1); id });
			let ev = Evidence { id, domain, target_id, owner: who.clone(), imgs: imgs.try_into().map_err(|_| Error::<T>::NotAuthorized)?, vids: vids.try_into().map_err(|_| Error::<T>::NotAuthorized)?, docs: docs.try_into().map_err(|_| Error::<T>::NotAuthorized)?, memo };
			Evidences::<T>::insert(id, &ev);
			EvidenceByTarget::<T>::insert((domain, target_id), id, ());
			Self::deposit_event(Event::EvidenceCommitted { id, domain, target_id, owner: who });
			Ok(())
		}

		/// 函数级中文注释：为目标链接已存在的证据（允许复用）；仅授权账户可调用。
		#[pallet::weight(10_000)]
		pub fn link(origin: OriginFor<T>, domain: u8, target_id: u64, id: u64) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let ns = T::EvidenceNsBytes::get();
			ensure!(<T as Config>::Authorizer::is_authorized(ns, &who), Error::<T>::NotAuthorized);
			ensure!(Evidences::<T>::contains_key(id), Error::<T>::NotFound);
			EvidenceByTarget::<T>::insert((domain, target_id), id, ());
			Self::deposit_event(Event::EvidenceLinked { domain, target_id, id });
			Ok(())
		}

		/// 函数级中文注释：取消目标与证据的链接；仅授权账户可调用。
		#[pallet::weight(10_000)]
		pub fn unlink(origin: OriginFor<T>, domain: u8, target_id: u64, id: u64) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let ns = T::EvidenceNsBytes::get();
			ensure!(<T as Config>::Authorizer::is_authorized(ns, &who), Error::<T>::NotAuthorized);
			EvidenceByTarget::<T>::remove((domain, target_id), id);
			Self::deposit_event(Event::EvidenceUnlinked { domain, target_id, id });
			Ok(())
		}
	}

	/// 授权适配接口：由 runtime 实现并桥接到 `pallet-authorizer`，以保持低耦合。
	pub trait EvidenceAuthorizer<AccountId> {
		/// 校验某账户是否在给定命名空间下被授权提交/链接证据
		fn is_authorized(ns: [u8; 8], who: &AccountId) -> bool;
	}

	/// 只读查询 trait 占位：供其他 pallet 低耦合读取证据（可在 runtime 或外部实现）。
	pub trait EvidenceProvider<AccountId> {
		/// 返回指定 ID 的证据；本 Pallet 不提供默认实现，避免类型推断问题。
		fn get(_id: u64) -> Option<()>;
	}
}


