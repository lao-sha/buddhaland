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
    use crate::weights::WeightInfo;
    // 基准模块在 pallet 外部声明；此处不在 proc-macro 输入中声明子模块，避免 E0658

    #[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
    pub enum Decision { Release, Refund, Partial(u16) } // bps

    /// 仲裁域路由接口：由 runtime 实现，根据域将仲裁请求路由到对应业务 pallet
    ///
    /// 设计目的：
    /// - 以 [u8;8] 域常量（通常与 PalletId 字节对齐）标识业务域
    /// - can_dispute：校验发起人是否有权对 (domain, id) 发起争议
    /// - apply_decision：按裁决对 (domain, id) 应用资金与状态变更（由各业务 pallet 内部完成）
    pub trait ArbitrationRouter<AccountId> {
        /// 校验是否允许发起争议
        fn can_dispute(domain: [u8; 8], who: &AccountId, id: u64) -> bool;
        /// 应用裁决（放款/退款/部分放款）
        fn apply_decision(domain: [u8; 8], id: u64, decision: Decision) -> DispatchResult;
    }

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_escrow::pallet::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type MaxEvidence: Get<u32>;
        type MaxCidLen: Get<u32>;
        /// 托管接口（调用释放/退款/部分分账）
        type Escrow: EscrowTrait<Self::AccountId, BalanceOf<Self>>;
        /// 权重信息
        type WeightInfo: weights::WeightInfo;
        /// 域路由：把仲裁请求路由到对应业务 pallet 的仲裁钩子
        type Router: ArbitrationRouter<Self::AccountId>;
    }

    pub type BalanceOf<T> = <<T as pallet_escrow::pallet::Config>::Currency as frame_support::traits::Currency<<T as frame_system::Config>::AccountId>>::Balance;
    

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// 争议登记：(domain, object_id) => ()
    #[pallet::storage]
    pub type Disputed<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat, [u8; 8],
        Blake2_128Concat, u64,
        (), OptionQuery
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// 发起争议事件（含域）
        Disputed { domain: [u8; 8], id: u64 },
        /// 完成裁决事件（含域）
        Arbitrated { domain: [u8; 8], id: u64, decision: u8, bps: Option<u16> },
    }

    #[pallet::error]
    pub enum Error<T> { AlreadyDisputed, NotDisputed }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// 发起仲裁：记录争议，证据 CID 存链（仅登记摘要/CID，不碰业务存储）
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::dispute(evidence.len() as u32))]
        pub fn dispute(origin: OriginFor<T>, domain: [u8; 8], id: u64, evidence: alloc::vec::Vec<BoundedVec<u8, T::MaxCidLen>>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            // 鉴权：由 Router 依据业务 pallet 规则判断是否允许发起（基准模式下跳过，便于构造场景）
            #[cfg(not(feature = "runtime-benchmarks"))]
            {
                ensure!(T::Router::can_dispute(domain, &who, id), Error::<T>::NotDisputed);
            }
            ensure!(Disputed::<T>::get(domain, id).is_none(), Error::<T>::AlreadyDisputed);
            Disputed::<T>::insert(domain, id, ());
            // 证据仅留 CID；如需可扩展附加存储（MVP 省略内容）
            Self::deposit_event(Event::Disputed { domain, id });
            Ok(())
        }
        /// 仲裁者裁决（白名单控制由 authorizer/forwarder 负责）
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::arbitrate())]
        pub fn arbitrate(origin: OriginFor<T>, domain: [u8; 8], id: u64, decision_code: u8, bps: Option<u16>) -> DispatchResult {
            let _arb = ensure_signed(origin)?;
            ensure!(Disputed::<T>::get(domain, id).is_some(), Error::<T>::NotDisputed);
            // 通过 Router 将裁决应用到对应域的业务 pallet
            let decision = match (decision_code, bps) {
                (0, _) => Decision::Release,
                (1, _) => Decision::Refund,
                (2, Some(p)) => Decision::Partial(p),
                _ => Decision::Refund,
            };
            T::Router::apply_decision(domain, id, decision.clone())?;
            let out = match decision { Decision::Release => (0, None), Decision::Refund => (1, None), Decision::Partial(p) => (2, Some(p)) };
            Self::deposit_event(Event::Arbitrated { domain, id, decision: out.0, bps: out.1 });
            Ok(())
        }
    }
}


