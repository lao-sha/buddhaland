#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use alloc::vec::Vec;
    use frame_support::{
        pallet_prelude::*,
        traits::Contains,
        dispatch::GetDispatchInfo,
        BoundedVec,
    };
    use frame_system::pallet_prelude::*;
    use sp_core::sr25519;
    use sp_runtime::{traits::{StaticLookup, Dispatchable}, RuntimeDebug};
    use frame_system::RawOrigin;
    use codec::Decode;

    /// Authorizer 适配接口：由 runtime 实现以对接 `pallet-authorizer`
    ///
    /// 注意：这是极简接口，仅用于本 MVP 示例。实际项目可拓展为预算预扣与结算。
    pub trait ForwarderAuthorizer<AccountId, Call> {
        /// 校验赞助者是否在给定命名空间下被允许代付
        fn is_sponsor_allowed(ns: [u8; 8], sponsor: &AccountId) -> bool;
        /// 校验该调用是否在允许范围（可包含参数上限检查）
        fn is_call_allowed(ns: [u8; 8], sponsor: &AccountId, call: &Call) -> bool;
    }

    /// 会话许可（简化版）：登录/开局时由用户主钱包签名，授权一个会话公钥在 TTL 内代签
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct SessionPermit<AccountId, BlockNumber> {
        /// 命名空间字节（与授权中心挂钩）
        pub ns: [u8; 8],
        /// 业务所有者账户（以该身份执行内层调用）
        pub owner: AccountId,
        /// 会话标识
        pub session_id: [u8; 16],
        /// 会话公钥（sr25519）
        pub session_pubkey: sr25519::Public,
        /// 过期高度（TTL）
        pub expires_at: BlockNumber,
    }

    /// 单笔元交易（简化版）：由会话私钥离线签名
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct MetaTx<Call, BlockNumber> {
        /// 命名空间
        pub ns: [u8; 8],
        /// 会话标识
        pub session_id: [u8; 16],
        /// 真实 RuntimeCall
        pub call: Call,
        /// 会话级防重放 nonce（严格递增）
        pub nonce: u64,
        /// 单笔过期高度
        pub valid_till: BlockNumber,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// 事件类型
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// 运行时聚合调用类型（用于封装真实调用）
        /// 要求其 PostInfo 与标准 `PostDispatchInfo` 一致，以便返回类型统一
        type RuntimeCall: Parameter
            + Dispatchable<
                RuntimeOrigin = Self::RuntimeOrigin,
                PostInfo = frame_support::dispatch::PostDispatchInfo,
            >
            + GetDispatchInfo;
        /// Authorizer 适配器
        type Authorizer: ForwarderAuthorizer<Self::AccountId, <Self as pallet::Config>::RuntimeCall>;
        /// 拒绝的调用（例如 utility::batch/dispatch_as 等）
        type ForbiddenCalls: Contains<<Self as pallet::Config>::RuntimeCall>;
        /// MetaTx 字节上限（避免滥用过大负载）
        type MaxMetaLen: Get<u32>;
        /// SessionPermit 字节上限
        type MaxPermitLen: Get<u32>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn session_of)]
    /// 已开启的会话：(owner, ns, session_id) -> SessionPermit
    pub type Sessions<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat, (T::AccountId, [u8; 8]),
        Blake2_128Concat, [u8; 16],
        SessionPermit<T::AccountId, BlockNumberFor<T>>, OptionQuery
    >;

    #[pallet::storage]
    #[pallet::getter(fn next_nonce)]
    /// 会话内 nonce 记录：(owner, ns, session_id) -> next_nonce
    pub type SessionNonce<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat, (T::AccountId, [u8; 8]),
        Blake2_128Concat, [u8; 16],
        u64, ValueQuery
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// 开启会话
        SessionOpened { owner: T::AccountId, ns: [u8; 8], session_id: [u8; 16] },
        /// 关闭会话
        SessionClosed { owner: T::AccountId, ns: [u8; 8], session_id: [u8; 16] },
        /// 成功转发
        Forwarded { owner: T::AccountId, sponsor: T::AccountId, ns: [u8; 8], session_id: [u8; 16] },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// 赞助者不在白名单
        SponsorNotAllowed,
        /// 调用未在允许范围
        CallNotAllowed,
        /// 会话不存在或已过期
        BadSession,
        /// Nonce 无效（非严格递增）
        BadNonce,
        /// 禁止的调用（如 batch/dispatch_as 等）
        ForbiddenCall,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// 开启会话：由赞助者代付将已签名的许可上链（本 MVP 省略许可签名校验，聚焦集成）
        ///
        /// 安全说明：生产环境应校验 SessionPermit 是由 `owner` 主钱包签名授权的。
        #[pallet::call_index(0)]
        #[pallet::weight(10_000)]
        pub fn open_session(origin: OriginFor<T>, permit_bytes: BoundedVec<u8, T::MaxPermitLen>) -> DispatchResult {
            let sponsor = ensure_signed(origin)?;
            // 解码离线许可
            let mut input = &permit_bytes[..];
            let permit: SessionPermit<T::AccountId, BlockNumberFor<T>> = SessionPermit::decode(&mut input)
                .map_err(|_| Error::<T>::BadSession)?;
            // authorizer：仅允许在白名单中的赞助者代付开启
            ensure!(T::Authorizer::is_sponsor_allowed(permit.ns, &sponsor), Error::<T>::SponsorNotAllowed);
            let now = frame_system::Pallet::<T>::block_number();
            ensure!(now <= permit.expires_at, Error::<T>::BadSession);
            Sessions::<T>::insert((permit.owner.clone(), permit.ns), permit.session_id, permit.clone());
            SessionNonce::<T>::insert((permit.owner.clone(), permit.ns), permit.session_id, 0u64);
            Self::deposit_event(Event::SessionOpened { owner: permit.owner, ns: permit.ns, session_id: permit.session_id });
            Ok(())
        }

        /// 关闭会话：由所有者主动撤销
        #[pallet::call_index(1)]
        #[pallet::weight(10_000)]
        pub fn close_session(origin: OriginFor<T>, ns: [u8; 8], session_id: [u8; 16]) -> DispatchResult {
            let owner = ensure_signed(origin)?;
            Sessions::<T>::remove((owner.clone(), ns), session_id);
            SessionNonce::<T>::remove((owner.clone(), ns), session_id);
            Self::deposit_event(Event::SessionClosed { owner, ns, session_id });
            Ok(())
        }

        /// 元交易转发：由赞助者签名付费，会话私钥对 MetaTx 离线签名（MVP 省略校验）
        ///
        /// 安全说明：生产环境应校验 `session_sig` 确实由 `session_pubkey` 对 `meta` 的 SCALE 编码签发。
        #[pallet::call_index(2)]
        #[pallet::weight(10_000)]
        pub fn forward(
            origin: OriginFor<T>,
            meta_bytes: BoundedVec<u8, T::MaxMetaLen>,
            _session_sig: Vec<u8>,
            owner: <T::Lookup as StaticLookup>::Source,
        ) -> DispatchResultWithPostInfo {
            let sponsor = ensure_signed(origin)?;
            let owner = T::Lookup::lookup(owner)?;
            // 解码 meta
            let mut input = &meta_bytes[..];
            let meta: MetaTx<<T as pallet::Config>::RuntimeCall, BlockNumberFor<T>> =
                MetaTx::decode(&mut input).map_err(|_| Error::<T>::BadSession)?;

            // 赞助者白名单
            ensure!(T::Authorizer::is_sponsor_allowed(meta.ns, &sponsor), Error::<T>::SponsorNotAllowed);

            // 会话存在与未过期
            let maybe = Sessions::<T>::get((owner.clone(), meta.ns), meta.session_id).ok_or(Error::<T>::BadSession)?;
            let now = frame_system::Pallet::<T>::block_number();
            ensure!(now <= maybe.expires_at && now <= meta.valid_till, Error::<T>::BadSession);

            // 禁止的调用过滤
            ensure!(!T::ForbiddenCalls::contains(&meta.call), Error::<T>::ForbiddenCall);

            // 范围校验
            ensure!(T::Authorizer::is_call_allowed(meta.ns, &sponsor, &meta.call), Error::<T>::CallNotAllowed);

            // Nonce 检查：严格递增
            let next = SessionNonce::<T>::get((owner.clone(), meta.ns), meta.session_id);
            ensure!(meta.nonce == next, Error::<T>::BadNonce);
            SessionNonce::<T>::insert((owner.clone(), meta.ns), meta.session_id, next.saturating_add(1));

            // 以用户身份执行真实调用
            let dispatch_result = meta.call.dispatch(RawOrigin::Signed(owner.clone()).into());

            // 事件（无论成功与否，外层费用由赞助者承担）
            if dispatch_result.is_ok() {
                Self::deposit_event(Event::Forwarded { owner, sponsor, ns: meta.ns, session_id: meta.session_id });
            }

            dispatch_result
        }
    }
}


