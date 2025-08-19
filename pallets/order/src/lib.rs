#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;
pub mod weights;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::Currency,
        BoundedVec,
    };
    use frame_system::pallet_prelude::*;
    use alloc::vec::Vec;
    use sp_core::H256;
    use sp_runtime::traits::{Saturating, SaturatedConversion};
    use pallet_escrow::pallet::Escrow as EscrowTrait;

    pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// 仲裁回调接口：供仲裁 pallet 触发实际放款/退款
    pub trait ArbitrationOrderHook<T: Config> {
        /// 可选：校验发起人是否可对该订单发起争议（买家/状态等）
        fn can_dispute(who: &T::AccountId, id: u64) -> bool;
        fn arbitrate_release(id: u64) -> DispatchResult;
        fn arbitrate_refund(id: u64) -> DispatchResult;
        fn arbitrate_partial(id: u64, bps: u16) -> DispatchResult;
    }

    /// Karma 增发适配接口（低耦合）：由 Runtime 将其实现桥接到具体的 `pallet-karma`。
    /// 函数级详细中文注释：
    /// - 为避免在本 Pallet 中直接依赖 `pallet_karma`，我们仅声明所需最小接口。
    /// - Runtime 侧通过 `impl KarmaMint for pallet_karma::Pallet<Runtime>` 完成桥接调用。
    pub trait KarmaMint<AccountId> {
        type Balance;
        fn gain(origin_caller: &AccountId, who: &AccountId, amount: Self::Balance, memo: Vec<u8>) -> DispatchResult;
    }

    #[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
    pub enum OrderStatus { Created, Paid, Assigned, InProgress, Submitted, Disputed, Released, Refunded, Closed }

    #[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
    pub struct Order<AccountId, Balance, BlockNumber> {
        pub buyer: AccountId,
        pub agent: Option<AccountId>,
        pub temple_id: u32,
        pub service_id: u32,
        pub qty: u32,
        pub locked: Balance,
        pub status: OrderStatus,
        pub decision_deadline: Option<BlockNumber>,
    }

    #[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(MaxCidLen, MaxImg, MaxVid))]
    pub struct OrderProof<MaxCidLen: Get<u32>, MaxImg: Get<u32>, MaxVid: Get<u32>> {
        pub imgs: BoundedVec<BoundedVec<u8, MaxCidLen>, MaxImg>,
        pub vids: BoundedVec<BoundedVec<u8, MaxCidLen>, MaxVid>,
        pub note_hash: Option<H256>,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type Currency: Currency<Self::AccountId>;
        type PalletIdGet: Get<frame_support::PalletId>; // 兼容保留（可去除）
        type PlatformAccount: Get<Self::AccountId>;
        type PlatformFeeBps: Get<u16>;
        type ConfirmTTL: Get<BlockNumberFor<Self>>;
        type MaxCidLen: Get<u32>;
        type MaxImg: Get<u32>;
        type MaxVid: Get<u32>;
        /// 托管接口（对接 pallet-escrow）
        type Escrow: pallet_escrow::pallet::Escrow<Self::AccountId, BalanceOf<Self>>;
        /// Karma 适配器：低耦合增发接口，由 Runtime 绑定到实际实现（例如 `pallet_karma::Pallet<Runtime>`）
        /// 函数级详细中文注释：
        /// - 通过本地 `KarmaMint` Trait 解耦，避免本 Pallet 对具体 Karma 实现产生编译期依赖。
        /// - 订单买家确认完成后，将调用 `Karma::gain` 为买家增发与订单金额等额（1:1）的 Karma。
        type Karma: KarmaMint<Self::AccountId, Balance = BalanceOf<Self>>;
        /// 权重信息
        type WeightInfo: weights::WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    pub type Orders<T: Config> = StorageMap<_, Blake2_128Concat, u64, Order<T::AccountId, BalanceOf<T>, BlockNumberFor<T>>, OptionQuery>;
    #[pallet::storage]
    pub type ProofOf<T: Config> = StorageMap<_, Blake2_128Concat, u64, OrderProof<T::MaxCidLen, T::MaxImg, T::MaxVid>, OptionQuery>;
    #[pallet::storage]
    pub type NextOrderId<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        OrderCreated { id: u64 },
        /// 函数级中文注释：代办接单事件（状态转为 Assigned）
        OrderAccepted { id: u64 },
        /// 函数级中文注释：代办开始执行事件（状态转为 InProgress）
        OrderStarted { id: u64 },
        ProofSubmitted { id: u64, img_count: u32, vid_count: u32 },
        BuyerConfirmed { id: u64 },
        AutoReleased { id: u64 },
        ReleasedToAgent { id: u64, to_agent: BalanceOf<T>, fee: BalanceOf<T> },
    }

    #[pallet::error]
    pub enum Error<T> {
        NotFound,
        BadState,
        EmptyProof,
        TooManyImages,
        TooManyVideos,
        NotBuyer,
        /// 非订单代办人
        NotAgent,
    }

    impl<T: Config> Pallet<T> {
        /// 将托管金额按“代办实收 + 平台费”分账，仅给代办
        fn payout_to_agent(id: u64, agent: &T::AccountId, total: BalanceOf<T>) -> DispatchResult {
            let bps: u128 = T::PlatformFeeBps::get() as u128;
            let tot: u128 = total.saturated_into::<u128>();
            let fee_u128 = tot.saturating_mul(bps) / 10_000u128;
            let fee: BalanceOf<T> = fee_u128.saturated_into();
            let to_agent: BalanceOf<T> = total.saturating_sub(fee);
            if !to_agent.is_zero() { T::Escrow::transfer_from_escrow(id, agent, to_agent)?; }
            if !fee.is_zero() { T::Escrow::transfer_from_escrow(id, &T::PlatformAccount::get(), fee)?; }
            Self::deposit_event(Event::ReleasedToAgent { id, to_agent, fee });
            Ok(())
        }
    }

    pub use crate::weights::WeightInfo;

    impl<T: Config> ArbitrationOrderHook<T> for Pallet<T> {
        fn can_dispute(who: &T::AccountId, id: u64) -> bool {
            if let Some(order) = Orders::<T>::get(id) {
                // 仅买家，且状态在可争议区间
                return order.buyer == *who && matches!(order.status, OrderStatus::Submitted | OrderStatus::InProgress | OrderStatus::Assigned);
            }
            false
        }
        /// 仲裁：全额放款（仅给代办，平台费分账）
        fn arbitrate_release(id: u64) -> DispatchResult {
            let mut order = Orders::<T>::get(id).ok_or(Error::<T>::NotFound)?;
            ensure!(matches!(order.status, OrderStatus::Submitted | OrderStatus::Disputed), Error::<T>::BadState);
            let agent = order.agent.clone().ok_or(Error::<T>::BadState)?;
            let amount = <T::Escrow as EscrowTrait<T::AccountId, BalanceOf<T>>>::amount_of(id);
            order.status = OrderStatus::Released;
            Orders::<T>::insert(id, &order);
            Self::payout_to_agent(id, &agent, amount)?;
            order.status = OrderStatus::Closed;
            Orders::<T>::insert(id, order);
            Ok(())
        }
        /// 仲裁：全额退款
        fn arbitrate_refund(id: u64) -> DispatchResult {
            let mut order = Orders::<T>::get(id).ok_or(Error::<T>::NotFound)?;
            ensure!(matches!(order.status, OrderStatus::Submitted | OrderStatus::Disputed), Error::<T>::BadState);
            let buyer = order.buyer.clone();
            <T::Escrow as EscrowTrait<T::AccountId, BalanceOf<T>>>::refund_all(id, &buyer)?;
            order.status = OrderStatus::Refunded;
            Orders::<T>::insert(id, &order);
            order.status = OrderStatus::Closed;
            Orders::<T>::insert(id, order);
            Ok(())
        }
        /// 仲裁：部分放款（bps/10000 给代办，其余退款买家）
        fn arbitrate_partial(id: u64, bps: u16) -> DispatchResult {
            let mut order = Orders::<T>::get(id).ok_or(Error::<T>::NotFound)?;
            ensure!(matches!(order.status, OrderStatus::Submitted | OrderStatus::Disputed), Error::<T>::BadState);
            let agent = order.agent.clone().ok_or(Error::<T>::BadState)?;
            let total = <T::Escrow as EscrowTrait<T::AccountId, BalanceOf<T>>>::amount_of(id);
            let tot_u128: u128 = total.saturated_into::<u128>();
            let bps_u128: u128 = bps as u128;
            let agent_u128 = tot_u128.saturating_mul(bps_u128) / 10_000u128;
            let agent_part: BalanceOf<T> = agent_u128.saturated_into();
            if !agent_part.is_zero() { Self::payout_to_agent(id, &agent, agent_part)?; }
            // 其余退款给买家
            let buyer = order.buyer.clone();
            <T::Escrow as EscrowTrait<T::AccountId, BalanceOf<T>>>::refund_all(id, &buyer)?;
            order.status = OrderStatus::Closed;
            Orders::<T>::insert(id, order);
            Ok(())
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// 代办接单：将调用者设为订单代办，订单状态从 Created → Assigned
        /// 函数级详细中文注释：
        /// - 前置条件：订单存在、状态为 `Created` 且尚未绑定代办。
        /// - 调用者：任意账户可尝试接单（实际项目中可接入资格校验/白名单）。
        /// - 后置效果：记录代办账户并推进状态，触发 `OrderAccepted` 事件。
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::accept_order())]
        pub fn accept_order(origin: OriginFor<T>, id: u64) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut order = Orders::<T>::get(id).ok_or(Error::<T>::NotFound)?;
            ensure!(matches!(order.status, OrderStatus::Created), Error::<T>::BadState);
            ensure!(order.agent.is_none(), Error::<T>::BadState);
            order.agent = Some(who);
            order.status = OrderStatus::Assigned;
            Orders::<T>::insert(id, &order);
            Self::deposit_event(Event::OrderAccepted { id });
            Ok(())
        }

        /// 代办开始执行：订单状态从 Assigned → InProgress
        /// 函数级详细中文注释：
        /// - 前置条件：订单存在、状态为 `Assigned`，且调用者必须为该订单的代办账户。
        /// - 后置效果：状态推进为 `InProgress`，触发 `OrderStarted` 事件。
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::start_order())]
        pub fn start_order(origin: OriginFor<T>, id: u64) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut order = Orders::<T>::get(id).ok_or(Error::<T>::NotFound)?;
            ensure!(matches!(order.status, OrderStatus::Assigned), Error::<T>::BadState);
            let agent = order.agent.clone().ok_or(Error::<T>::BadState)?;
            ensure!(agent == who, Error::<T>::NotAgent);
            order.status = OrderStatus::InProgress;
            Orders::<T>::insert(id, &order);
            Self::deposit_event(Event::OrderStarted { id });
            Ok(())
        }
        /// 创建订单（最小骨架：仅示例必要字段）
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::create_order())]
        pub fn create_order(origin: OriginFor<T>, temple_id: u32, service_id: u32, qty: u32, locked: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let id = NextOrderId::<T>::mutate(|x| { let id=*x; *x=id.saturating_add(1); id });
            Orders::<T>::insert(id, Order { buyer: who, agent: None, temple_id, service_id, qty, locked, status: OrderStatus::Created, decision_deadline: None });
            // 调用托管接口：从买家转入托管账户并记录
            <T::Escrow as EscrowTrait<T::AccountId, BalanceOf<T>>>::lock_from(&Orders::<T>::get(id).unwrap().buyer, id, locked)?;
            Self::deposit_event(Event::OrderCreated { id });
            Ok(())
        }

        /// 代办提交凭证（IPFS CID），订单置为 Submitted，进入买家确认期
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::submit_order_proof(imgs.len() as u32 + vids.len() as u32))]
        pub fn submit_order_proof(origin: OriginFor<T>, id: u64, imgs: Vec<BoundedVec<u8, T::MaxCidLen>>, vids: Vec<BoundedVec<u8, T::MaxCidLen>>, note_hash: Option<H256>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut order = Orders::<T>::get(id).ok_or(Error::<T>::NotFound)?;
            // 仅订单代办可提交，且必须已处于 InProgress 状态
            let agent = order.agent.clone().ok_or(Error::<T>::BadState)?;
            ensure!(agent == who, Error::<T>::NotAgent);
            ensure!(matches!(order.status, OrderStatus::InProgress), Error::<T>::BadState);
            ensure!(!(imgs.is_empty() && vids.is_empty()), Error::<T>::EmptyProof);
            ensure!(imgs.len() as u32 <= T::MaxImg::get(), Error::<T>::TooManyImages);
            ensure!(vids.len() as u32 <= T::MaxVid::get(), Error::<T>::TooManyVideos);
            // 函数级中文注释：
            // 为避免在调用 `try_into()` 后变量被移动导致后续无法读取长度（E0382），
            // 先记录提交的图片与视频数量，再执行消耗所有权的转换。
            let img_count: u32 = imgs.len() as u32;
            let vid_count: u32 = vids.len() as u32;
            let proof = OrderProof {
                imgs: imgs.try_into().map_err(|_| Error::<T>::TooManyImages)?,
                vids: vids.try_into().map_err(|_| Error::<T>::TooManyVideos)?,
                note_hash,
            };
            ProofOf::<T>::insert(id, proof);
            order.status = OrderStatus::Submitted;
            order.decision_deadline = Some(<frame_system::Pallet<T>>::block_number() + T::ConfirmTTL::get());
            Orders::<T>::insert(id, order);
            Self::deposit_event(Event::ProofSubmitted { id, img_count, vid_count });
            Ok(())
        }

        /// 买家确认完成 → 放款给代办 + 平台分账 + 为买家增发 Karma（1:1，与订单金额相等）
        /// 函数级详细中文注释：
        /// - 前置条件：仅订单买家可调用，且订单状态必须为 `Submitted`（代办已提交凭证，处于确认期）。
        /// - 资金流程：调用托管模块按平台费比例分账，净额打给代办（寺庙不收款）。
        /// - 奖励流程：放款成功后，按订单托管金额 1:1 为买家增发 Karma。
        ///   - 调用方式：以 `PlatformAccount` 作为“授权调用者”调用 `T::Karma::gain`；
        ///   - 授权与失败：需预先在 `pallet-authorizer` 为 `Karma` 的命名空间授权该调用者；
        ///     若未授权或备注过长导致失败，整个交易回滚，确保原子性与一致性。
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::confirm_done_by_buyer())]
        pub fn confirm_done_by_buyer(origin: OriginFor<T>, id: u64) -> DispatchResult {
            let buyer = ensure_signed(origin)?;
            let mut order = Orders::<T>::get(id).ok_or(Error::<T>::NotFound)?;
            ensure!(order.buyer == buyer, Error::<T>::NotBuyer);
            ensure!(matches!(order.status, OrderStatus::Submitted), Error::<T>::BadState);
            let agent = order.agent.clone().ok_or(Error::<T>::BadState)?;
            let amount = <T::Escrow as EscrowTrait<T::AccountId, BalanceOf<T>>>::amount_of(id);
            order.status = OrderStatus::Released;
            Orders::<T>::insert(id, &order);
            Self::payout_to_agent(id, &agent, amount)?;
            // 为买家增发与订单金额等额的 Karma（1:1）。
            // 以平台账户作为授权调用者，备注包含订单标识，便于链下审计。
            {
                use alloc::vec::Vec;
                let caller = T::PlatformAccount::get();
                let mut memo: Vec<u8> = b"order:".to_vec();
                memo.extend_from_slice(&id.to_le_bytes());
                <T as Config>::Karma::gain(&caller, &buyer, amount, memo)?;
            }
            order.status = OrderStatus::Closed;
            Orders::<T>::insert(id, order);
            Self::deposit_event(Event::BuyerConfirmed { id });
            Ok(())
        }

        /// 超时自动放款（任何人可触发）：Submitted 且过期未争议
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::finalize_expired())]
        pub fn finalize_expired(origin: OriginFor<T>, id: u64) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            let mut order = Orders::<T>::get(id).ok_or(Error::<T>::NotFound)?;
            ensure!(matches!(order.status, OrderStatus::Submitted), Error::<T>::BadState);
            let ddl = order.decision_deadline.ok_or(Error::<T>::BadState)?;
            ensure!(<frame_system::Pallet<T>>::block_number() >= ddl, Error::<T>::BadState);
            let agent = order.agent.clone().ok_or(Error::<T>::BadState)?;
            let amount = <T::Escrow as EscrowTrait<T::AccountId, BalanceOf<T>>>::amount_of(id);
            order.status = OrderStatus::Released;
            Orders::<T>::insert(id, &order);
            Self::payout_to_agent(id, &agent, amount)?;
            order.status = OrderStatus::Closed;
            Orders::<T>::insert(id, order);
            Self::deposit_event(Event::AutoReleased { id });
            Ok(())
        }
    }
}


