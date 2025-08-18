#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::{EnsureOrigin, Get},
        BoundedVec,
    };
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use codec::Decode;

    /// 冥想会话摘要头（仅存聚合数据与承诺，不含原始脑波）
    #[derive(Encode, Decode, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(MaxOffchainLen))]
    pub struct SessionHeader<AccountId, BlockNumber, MaxOffchainLen: Get<u32>> {
        /// 业务命名空间（与授权中心/前端域隔离一致）
        pub ns: [u8; 8],
        /// 会话所属账户
        pub owner: AccountId,
        /// 设备标识（H256 哈希，不暴露序列号明文）
        pub device_id: H256,
        /// 是否使用了头环
        pub used_headband: bool,
        /// 开始区块与持续秒数（近似）
        pub start_at: BlockNumber,
        pub duration_secs: u32,
        /// 质量指标（简化）：有效分钟数与质量百分比
        pub valid_minutes: u16,
        pub quality_pct: u8,
        /// 摘要哈希（对规范化摘要/指标做 SCALE 编码后哈希）
        pub summary_hash: H256,
        /// 原始数据 Merkle 根（可选）
        pub raw_root: Option<H256>,
        /// 链下链接（IPFS CID/私有存储索引）的哈希（避免明文泄露）
        pub offchain_link_hash: BoundedVec<u8, MaxOffchainLen>,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_device::Config + pallet_mining::Config {
        /// 事件类型
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// 管理员 Origin（参数治理/紧急开关）
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        /// 允许的最大 offchain 链接哈希长度
        type MaxOffchainLen: Get<u32>;
        /// 是否强制头环（全局开关）
        type RequireHeadband: Get<bool>;
        /// 是否强制设备绑定（头环模式下）
        type RequireBinding: Get<bool>;
        /// 是否强制设备签名（留作扩展；MVP 不校验）
        type RequireDeviceSignature: Get<bool>;
        /// Header 编码字节上限（避免参数过大）
        type MaxHeaderLen: Get<u32>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// (owner, ns, session_id) -> header
    #[pallet::storage]
    pub type Sessions<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat, (T::AccountId, [u8; 8]),
        Blake2_128Concat, [u8; 16],
        SessionHeader<T::AccountId, BlockNumberFor<T>, T::MaxOffchainLen>, OptionQuery
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// 提交会话摘要
        SessionSubmitted { owner: T::AccountId, ns: [u8; 8], session_id: [u8; 16], device_id: H256 },
        /// 已发放奖励
        SessionRewarded { owner: T::AccountId, device_id: H256 },
    }

    #[pallet::error]
    pub enum Error<T> {
        DuplicatedSession,
        TooShortDuration,
        DeviceNotActive,
        NotBound,
        HeadbandRequired,
        InvalidQuality,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// 提交会话摘要并触发挖矿（内部将调用 pallet-device 与 pallet-mining 验证与发奖）
        /// - 仅存摘要/承诺，不含原始脑波；设备签名在 MVP 暂不校验
        #[pallet::call_index(0)]
        #[pallet::weight(10_000)]
        pub fn submit_session(
            origin: OriginFor<T>,
            session_id: [u8; 16],
            header_bytes: BoundedVec<u8, T::MaxHeaderLen>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut input = &header_bytes[..];
            let header: SessionHeader<T::AccountId, BlockNumberFor<T>, T::MaxOffchainLen> =
                SessionHeader::decode(&mut input).map_err(|_| Error::<T>::InvalidQuality)?;
            ensure!(who == header.owner, Error::<T>::DuplicatedSession);
            ensure!(header.duration_secs >= 30, Error::<T>::TooShortDuration);
            ensure!(Sessions::<T>::get((header.owner.clone(), header.ns), session_id).is_none(), Error::<T>::DuplicatedSession);

            // 设备校验（当 used_headband 或强制要求时）
            if header.used_headband || T::RequireHeadband::get() {
                // 设备活跃
                let dev = pallet_device::Pallet::<T>::devices(header.device_id).ok_or(Error::<T>::DeviceNotActive)?;
                if !matches!(dev.status, pallet_device::pallet::DeviceStatus::Active) { return Err(Error::<T>::DeviceNotActive.into()); }
                // 绑定校验（可选）
                if T::RequireBinding::get() {
                    let bound = pallet_device::Pallet::<T>::device_owner_of(header.device_id).ok_or(Error::<T>::NotBound)?;
                    ensure!(bound == who, Error::<T>::NotBound);
                }
            }

            // 存储摘要
            let ns = header.ns;
            let device_id = header.device_id;
            let valid_minutes = header.valid_minutes;
            let quality_pct = header.quality_pct;
            Sessions::<T>::insert((who.clone(), ns), session_id, header);
            Self::deposit_event(Event::SessionSubmitted { owner: who.clone(), ns, session_id, device_id });

            // 触发挖矿发奖（通过内部接口）
            // 提示：需在 runtime 将“矿工模块账户”加入 mining 命名空间白名单
            let module_caller = who.clone(); // MVP：暂以用户自身作为 caller（生产应使用模块账户）
            <pallet_mining::Pallet<T> as pallet_mining::pallet::MiningInterface<
                T::AccountId,
                pallet_mining::pallet::BalanceOf<T>,
            >>::award_by(&module_caller, &who, device_id, true, valid_minutes, quality_pct)?;
            Self::deposit_event(Event::SessionRewarded { owner: who, device_id });

            Ok(())
        }
    }
}


