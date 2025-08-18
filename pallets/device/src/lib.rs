#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ReservableCurrency, EnsureOrigin, Get},
        BoundedVec,
    };
    use frame_system::pallet_prelude::*;
    use sp_core::{H256, sr25519, hashing};
    use sp_runtime::RuntimeDebug;

    /// 设备唯一性等级
    #[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum UniquenessLevel { Weak, Medium, Strong }

    /// 设备状态
    #[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum DeviceStatus { Active, Suspended, Revoked }

    /// 设备记录
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct DeviceRecord<AccountId, BlockNumber> {
        /// 厂商账户
        pub manufacturer: AccountId,
        /// 设备公钥（中/强唯一）
        pub pubkey: Option<sr25519::Public>,
        /// 状态
        pub status: DeviceStatus,
        /// 唯一性等级
        pub level: UniquenessLevel,
        /// 注册区块
        pub registered_at: BlockNumber,
        /// 设备规格/证书等元信息哈希
        pub meta_hash: Option<H256>,
        /// 厂商证书哈希（可选）
        pub cert_hash: Option<H256>,
    }

    /// 挑战结构
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct Challenge<BlockNumber> {
        pub nonce: H256,
        pub expires_at: BlockNumber,
    }

    pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// 事件类型
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// 管理员 Origin（用于厂商白名单与吊销等）
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        /// 代币接口（用于押金，可选）
        type Currency: ReservableCurrency<Self::AccountId>;
        /// 每账户可绑定的最大设备数
        type MaxDevicesPerOwner: Get<u32>;
        /// 挑战有效期（区块）
        type ChallengeTtl: Get<BlockNumberFor<Self>>;
        /// 设备注册最小押金（0 表示关闭押金）
        type MinRegisterDeposit: Get<BalanceOf<Self>>;
        /// 链上描述/证书锚点最大长度（字节）
        type MaxMetaLen: Get<u32>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn devices)]
    /// 设备主表：DeviceId(H256) -> 设备记录
    pub type Devices<T: Config> = StorageMap<
        _, Blake2_128Concat, H256,
        DeviceRecord<T::AccountId, BlockNumberFor<T>>, OptionQuery
    >;

    #[pallet::storage]
    #[pallet::getter(fn device_owner_of)]
    /// 设备当前绑定者：DeviceId -> Owner
    pub type DeviceOwnerOf<T: Config> = StorageMap<_, Blake2_128Concat, H256, T::AccountId, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn devices_of)]
    /// 账户名下设备集合：Owner -> BoundedVec<DeviceId>
    pub type DevicesOf<T: Config> = StorageMap<
        _, Blake2_128Concat, T::AccountId,
        BoundedVec<H256, T::MaxDevicesPerOwner>, ValueQuery
    >;

    #[pallet::storage]
    #[pallet::getter(fn pending_challenge)]
    /// 绑定挑战：(Owner, DeviceId) -> Challenge
    pub type PendingChallenges<T: Config> = StorageDoubleMap<
        _, Blake2_128Concat, T::AccountId,
        Blake2_128Concat, H256,
        Challenge<BlockNumberFor<T>>, OptionQuery
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// 注册成功
        HeadbandRegistered { device_id: H256, manufacturer: T::AccountId, level_code: u8 },
        /// 升级成功
        HeadbandPromoted { device_id: H256, new_level_code: u8 },
        /// 吊销/暂停/恢复
        HeadbandRevoked { device_id: H256 },
        HeadbandSuspended { device_id: H256 },
        HeadbandResumed { device_id: H256 },
        /// 绑定流程
        BindChallengeOpened { owner: T::AccountId, device_id: H256, nonce: H256, expires_at: BlockNumberFor<T> },
        HeadbandBound { owner: T::AccountId, device_id: H256 },
        HeadbandUnbound { owner: T::AccountId, device_id: H256 },
    }

    #[pallet::error]
    pub enum Error<T> {
        ManufacturerOnly,
        DeviceExists,
        DeviceNotFound,
        DeviceNotActive,
        AlreadyBound,
        NotBound,
        OwnerQuotaExceeded,
        ChallengeNotFound,
        ChallengeExpired,
        InvalidLevel,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// 注册设备（弱/中/强唯一）；不上链明文序列号，仅用 DeviceId(H256)
        /// - 若配置押金>0，可在此保留押金（MVP 略）
        #[pallet::call_index(0)]
        #[pallet::weight(10_000)]
        pub fn register_headband(
            origin: OriginFor<T>,
            device_id: H256,
            pubkey: Option<sr25519::Public>,
            meta_hash: Option<H256>,
            cert_hash: Option<H256>,
            level_code: u8,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let level = match level_code { 0 => UniquenessLevel::Weak, 1 => UniquenessLevel::Medium, 2 => UniquenessLevel::Strong, _ => return Err(Error::<T>::InvalidLevel.into()) };
            // 管理员或厂商账户：MVP 允许任意签名者作为 manufacturer，生产版可用 AdminOrigin 审批厂商白名单
            ensure!(Devices::<T>::get(device_id).is_none(), Error::<T>::DeviceExists);
            let now = frame_system::Pallet::<T>::block_number();
            let rec = DeviceRecord { manufacturer: who.clone(), pubkey, status: DeviceStatus::Active, level, registered_at: now, meta_hash, cert_hash };
            Devices::<T>::insert(device_id, rec);
            Self::deposit_event(Event::HeadbandRegistered { device_id, manufacturer: who, level_code });
            Ok(())
        }

        /// 开启绑定挑战：生成一次性 nonce 与过期高度
        #[pallet::call_index(1)]
        #[pallet::weight(10_000)]
        pub fn open_bind_challenge(origin: OriginFor<T>, device_id: H256) -> DispatchResult {
            let owner = ensure_signed(origin)?;
            let dev = Devices::<T>::get(device_id).ok_or(Error::<T>::DeviceNotFound)?;
            ensure!(matches!(dev.status, DeviceStatus::Active), Error::<T>::DeviceNotActive);
            let mut nonce_bytes = [0u8; 32];
            // 简单伪随机：使用 (owner, device_id, block_number) 哈希；生产建议链上随机性或链下生成
            let seed = (owner.encode(), device_id, frame_system::Pallet::<T>::block_number()).using_encoded(hashing::blake2_256);
            nonce_bytes.copy_from_slice(&seed);
            let nonce = H256::from(nonce_bytes);
            let expires_at = frame_system::Pallet::<T>::block_number() + T::ChallengeTtl::get();
            PendingChallenges::<T>::insert(&owner, device_id, Challenge { nonce, expires_at });
            Self::deposit_event(Event::BindChallengeOpened { owner, device_id, nonce, expires_at });
            Ok(())
        }

        /// 绑定设备：MVP 不做设备签名校验；生产应验证设备公钥对 challenge 的签名
        #[pallet::call_index(2)]
        #[pallet::weight(10_000)]
        pub fn bind_headband(origin: OriginFor<T>, device_id: H256) -> DispatchResult {
            let owner = ensure_signed(origin)?;
            let dev = Devices::<T>::get(device_id).ok_or(Error::<T>::DeviceNotFound)?;
            ensure!(matches!(dev.status, DeviceStatus::Active), Error::<T>::DeviceNotActive);
            ensure!(DeviceOwnerOf::<T>::get(device_id).is_none(), Error::<T>::AlreadyBound);
            // 校验挑战存在且未过期
            let ch = PendingChallenges::<T>::get(&owner, device_id).ok_or(Error::<T>::ChallengeNotFound)?;
            ensure!(frame_system::Pallet::<T>::block_number() <= ch.expires_at, Error::<T>::ChallengeExpired);
            // 更新绑定关系
            DevicesOf::<T>::try_mutate(&owner, |vec| -> DispatchResult {
                if vec.try_push(device_id).is_err() { return Err(Error::<T>::OwnerQuotaExceeded.into()); }
                Ok(())
            })?;
            DeviceOwnerOf::<T>::insert(device_id, &owner);
            PendingChallenges::<T>::remove(&owner, device_id);
            Self::deposit_event(Event::HeadbandBound { owner, device_id });
            Ok(())
        }

        /// 解绑设备
        #[pallet::call_index(3)]
        #[pallet::weight(10_000)]
        pub fn unbind_headband(origin: OriginFor<T>, device_id: H256) -> DispatchResult {
            let owner = ensure_signed(origin)?;
            let curr = DeviceOwnerOf::<T>::get(device_id).ok_or(Error::<T>::NotBound)?;
            ensure!(curr == owner, Error::<T>::NotBound);
            DeviceOwnerOf::<T>::remove(device_id);
            DevicesOf::<T>::mutate(&owner, |vec| { if let Some(pos) = vec.iter().position(|x| *x == device_id) { vec.swap_remove(pos); } });
            Self::deposit_event(Event::HeadbandUnbound { owner, device_id });
            Ok(())
        }
    }
}


