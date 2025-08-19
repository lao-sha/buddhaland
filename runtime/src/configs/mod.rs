// This is free and unencumbered software released into the public domain.
//
// Anyone is free to copy, modify, publish, use, compile, sell, or
// distribute this software, either in source code form or as a compiled
// binary, for any purpose, commercial or non-commercial, and by any
// means.
//
// In jurisdictions that recognize copyright laws, the author or authors
// of this software dedicate any and all copyright interest in the
// software to the public domain. We make this dedication for the benefit
// of the public at large and to the detriment of our heirs and
// successors. We intend this dedication to be an overt act of
// relinquishment in perpetuity of all present and future rights to this
// software under copyright law.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS BE LIABLE FOR ANY CLAIM, DAMAGES OR
// OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
// ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
// OTHER DEALINGS IN THE SOFTWARE.
//
// For more information, please refer to <http://unlicense.org>

// Substrate and Polkadot dependencies
use frame_support::{
	derive_impl, parameter_types,
	traits::{ConstBool, ConstU128, ConstU32, ConstU64, ConstU8, VariantCountOf},
	weights::{
		constants::{RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND},
		IdentityFee, Weight,
	},
};
use frame_system::limits::{BlockLength, BlockWeights};
use alloc::vec::Vec; // 为 KarmaMintAdapter::gain 的 memo Vec 引入作用域
use pallet_transaction_payment::{ConstFeeMultiplier, FungibleAdapter, Multiplier};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_runtime::{traits::One, Perbill};
use sp_version::RuntimeVersion;

// 引入以区块数表示的一天常量，用于配置授权中心的投票期（AuthorizerVotingPeriod）
use crate::DAYS;
// 引入以区块数表示的一分钟常量，用于设备挑战 TTL 等时间参数
use crate::MINUTES;
// 引入余额单位常量
use crate::MILLI_UNIT;

// Local module imports
use super::{
	AccountId, Aura, Balance, Balances, Block, BlockNumber, Hash, Nonce, PalletInfo, Runtime,
	RuntimeCall, RuntimeEvent, RuntimeFreezeReason, RuntimeHoldReason, RuntimeOrigin, RuntimeTask,
	System, EXISTENTIAL_DEPOSIT, SLOT_DURATION, VERSION,
};

// ===== Forwarder 集成所需的适配与类型 =====
use pallet_forwarder::ForwarderAuthorizer;

/// Authorizer 适配器：把 `pallet-authorizer` 的接口桥接给 `pallet-forwarder`
pub struct AuthorizerAdapter;
impl ForwarderAuthorizer<AccountId, RuntimeCall> for AuthorizerAdapter {
	/// 校验赞助者是否在命名空间下被允许
	fn is_sponsor_allowed(ns: [u8; 8], sponsor: &AccountId) -> bool {
		pallet_authorizer::Pallet::<Runtime>::is_authorized(pallet_authorizer::pallet::Namespace(ns), sponsor)
	}

	/// 校验调用是否在允许范围（基于命名空间 + 具体 Call 变体匹配）
	fn is_call_allowed(ns: [u8; 8], _sponsor: &AccountId, call: &RuntimeCall) -> bool {
		let order_ns = OrderNsBytes::get();
		let device_ns = DeviceNsBytes::get();
		let meditation_ns = MeditationNsBytes::get();
		match (ns, call) {
			// 订单域：仅允许下单与履约关键路径
			(n, RuntimeCall::Order(inner)) if n == order_ns => matches!(
				inner,
				pallet_order::Call::create_order { .. }
				| pallet_order::Call::submit_order_proof { .. }
				| pallet_order::Call::confirm_done_by_buyer { .. }
				| pallet_order::Call::finalize_expired { .. }
			),
			// 设备域：绑定相关
			(n, RuntimeCall::Device(inner)) if n == device_ns => matches!(
				inner,
				pallet_device::Call::open_bind_challenge { .. } | pallet_device::Call::bind_headband { .. }
			),
			// 冥想域：仅提交会话摘要
			(n, RuntimeCall::Meditation(inner)) if n == meditation_ns => matches!(
				inner,
				pallet_meditation::Call::submit_session { .. }
			),
			// 仲裁域：允许提交争议与裁决（可叠加白名单控制仲裁者）
			(n, RuntimeCall::Arbitration(inner)) if n == ArbitrationNsBytes::get() => matches!(
				inner,
				pallet_arbitration::Call::dispute { .. } | pallet_arbitration::Call::arbitrate { .. }
			),
			_ => false,
		}
	}
}

/// 禁止调用集合（MVP：空集）。可在后续版本中拒绝 utility::batch/dispatch_as 等逃逸方法。
pub struct ForbidEscapeCalls;
impl frame_support::traits::Contains<RuntimeCall> for ForbidEscapeCalls {
    fn contains(call: &RuntimeCall) -> bool {
        // 禁用可能逃逸范围或高权限入口（根据是否引入相应 pallet 可调整）
        matches!(
            call,
            RuntimeCall::Sudo(_) // 禁止 sudo
        )
    }
}
parameter_types! {
	pub const AuthorizerMinDeposit: Balance = EXISTENTIAL_DEPOSIT;
	pub const AuthorizerVotingPeriod: BlockNumber = DAYS; // 约一天
}

impl pallet_authorizer::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type MinDeposit = AuthorizerMinDeposit;
	type VotingPeriod = AuthorizerVotingPeriod;
	type AdminOrigin = frame_system::EnsureRoot<AccountId>;
}


const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	pub const Version: RuntimeVersion = VERSION;

	/// We allow for 2 seconds of compute with a 6 second average block time.
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::with_sensible_defaults(
		Weight::from_parts(2u64 * WEIGHT_REF_TIME_PER_SECOND, u64::MAX),
		NORMAL_DISPATCH_RATIO,
	);
	pub RuntimeBlockLength: BlockLength = BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub const SS58Prefix: u8 = 42;
}

/// The default types are being injected by [`derive_impl`](`frame_support::derive_impl`) from
/// [`SoloChainDefaultConfig`](`struct@frame_system::config_preludes::SolochainDefaultConfig`),
/// but overridden as needed.
#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig)]
impl frame_system::Config for Runtime {
	/// The block type for the runtime.
	type Block = Block;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = RuntimeBlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = RuntimeBlockLength;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Version of the runtime.
	type Version = Version;
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = ConstU32<32>;
	type AllowMultipleBlocksPerSlot = ConstBool<false>;
	type SlotDuration = pallet_aura::MinimumPeriodTimesTwo<Runtime>;
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;

	type WeightInfo = ();
	type MaxAuthorities = ConstU32<32>;
	type MaxNominators = ConstU32<0>;
	type MaxSetIdSessionEntries = ConstU64<0>;

	type KeyOwnerProof = sp_core::Void;
	type EquivocationReportSystem = ();
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
	type WeightInfo = ();
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = VariantCountOf<RuntimeFreezeReason>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type DoneSlashHandler = ();
}

parameter_types! {
	pub FeeMultiplier: Multiplier = Multiplier::one();
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = FungibleAdapter<Balances, ()>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightToFee = IdentityFee<Balance>;
	type LengthToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
	type WeightInfo = pallet_transaction_payment::weights::SubstrateWeight<Runtime>;
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

/// Configure the pallet-template in pallets/template.
impl pallet_template::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_template::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const KarmaHistoryMaxLen: u32 = 1000;
	pub const KarmaMemoMaxLen: u32 = 128;
}

impl pallet_karma::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type KarmaBalance = Balance;
	// Use block number as Moment for lightweight time tracking
	// type Moment = BlockNumber; // removed, not used anymore
	type HistoryMaxLen = KarmaHistoryMaxLen;
	type MaxMemoLen = KarmaMemoMaxLen;
	// Karma 授权命名空间常量配置（以 8 字节固定标识绑定授权中心）
	type AuthorizerNamespace = KarmaNsBytes;
}

// ===== pallet-forwarder 配置实现 =====
impl pallet_forwarder::Config for Runtime {
	/// 事件类型
	type RuntimeEvent = RuntimeEvent;
	/// 运行时聚合调用类型（作为元交易内层调用）
	type RuntimeCall = RuntimeCall;
	/// Authorizer 适配器（桥接到 pallet-authorizer）
	type Authorizer = AuthorizerAdapter;
	/// 禁止调用集合（MVP：为空集）
	type ForbiddenCalls = ForbidEscapeCalls;
	/// 字节上限（根据业务情况调整）
	type MaxMetaLen = frame_support::traits::ConstU32<8192>;
	type MaxPermitLen = frame_support::traits::ConstU32<512>;
}

// ===== pallet-device 配置 =====
parameter_types! {
    pub const MaxDevicesPerOwner: u32 = 8;
    pub const ChallengeTtl: BlockNumber = 10 * MINUTES; // 约 10 分钟
    pub const MinRegisterDeposit: Balance = 0;
    pub const DeviceMaxMetaLen: u32 = 256;
}

impl pallet_device::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AdminOrigin = frame_system::EnsureRoot<AccountId>;
    type Currency = Balances;
    type MaxDevicesPerOwner = MaxDevicesPerOwner;
    type ChallengeTtl = ChallengeTtl;
    type MinRegisterDeposit = MinRegisterDeposit;
    type MaxMetaLen = DeviceMaxMetaLen;
}

// ===== pallet-mining 配置 =====
parameter_types! {
    pub const MiningMaxSessionMinutes: u16 = 180; // 单会话最多计 3h
    pub const MiningBasePerMinute: Balance = 1 * MILLI_UNIT; // 每分钟 0.001 BUD（示例）
    pub const MiningDailyCapPerDevice: Balance = 200 * MILLI_UNIT;
    pub const MiningDailyCapPerAccount: Balance = 500 * MILLI_UNIT;
    pub const MiningDailyCapGlobal: Balance = 100_000 * MILLI_UNIT;
}

// 冥想挖矿在授权中心的命名空间
parameter_types! {
    pub const MiningNsBytes: [u8; 8] = *b"mining__";
}

// 为 mining 提供授权适配器：桥接到 pallet-authorizer
pub struct MiningAuthorizerAdapter;
impl pallet_mining::pallet::MiningAuthorizer<AccountId> for MiningAuthorizerAdapter {
    fn is_authorized(caller: &AccountId) -> bool {
        let ns = MiningNsBytes::get();
        pallet_authorizer::Pallet::<Runtime>::is_authorized(pallet_authorizer::pallet::Namespace(ns), caller)
    }
}

impl pallet_mining::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type AdminOrigin = frame_system::EnsureRoot<AccountId>;
    type Authorizer = MiningAuthorizerAdapter;
    type MaxSessionMinutes = MiningMaxSessionMinutes;
    type DailyCapPerDevice = MiningDailyCapPerDevice;
    type DailyCapPerAccount = MiningDailyCapPerAccount;
    type DailyCapGlobal = MiningDailyCapGlobal;
    type BaseBudPerMinute = MiningBasePerMinute;
}

// ===== pallet-meditation 配置 =====
parameter_types! {
    pub const MeditationMaxOffchainLen: u32 = 96;
    pub const MeditationRequireHeadband: bool = true;
    pub const MeditationRequireBinding: bool = true;
    pub const MeditationRequireDeviceSig: bool = false; // MVP 暂不检验
}

// ===== 会话许可命名空间常量（用于 forwarder + authorizer） =====
parameter_types! {
    pub const OrderNsBytes: [u8; 8] = *b"order___";
    pub const DeviceNsBytes: [u8; 8] = *b"device__";
    pub const MeditationNsBytes: [u8; 8] = *b"meditate"; // 8字节
    pub const ArbitrationNsBytes: [u8; 8] = *b"arb___ _"; // 8字节
}

impl pallet_meditation::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AdminOrigin = frame_system::EnsureRoot<AccountId>;
    type MaxOffchainLen = MeditationMaxOffchainLen;
    type RequireHeadband = MeditationRequireHeadband;
    type RequireBinding = MeditationRequireBinding;
    type RequireDeviceSignature = MeditationRequireDeviceSig;
    type MaxHeaderLen = frame_support::traits::ConstU32<512>;
}

// ===== temple/agent/order 配置（MVP：仅最小实现所需） =====
parameter_types! {
    pub const MaxPriceTiers: u32 = 8;
    pub const MaxCalendarSlots: u32 = 64;
}
impl pallet_temple::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type MaxPriceTiers = MaxPriceTiers;
    type MaxCalendar = MaxCalendarSlots;
    // 价格/金额统一使用链上 Balance 类型
    type Balance = Balance;
}

parameter_types! {
    pub const AgentMaxSkills: u32 = 16;
    pub const AgentMaxCalendar: u32 = 64;
}
impl pallet_agent::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type MaxSkills = AgentMaxSkills;
    type MaxCalendar = AgentMaxCalendar;
}

// ===== ritual/cemetery/deceased 运行时参数占位（可按需调整） =====
parameter_types! {
    pub const RitualNsBytes: [u8; 8] = *b"ritual__";
    pub const RitualMaxSpecs: u32 = 1024;
    pub const RitualMaxSpecsPerKind: u32 = 128;
    pub const RitualMaxSpecsPerProvider: u32 = 128;
    pub const RitualMaxMemoLen: u32 = 128;
    pub const RitualMaxCidLen: u32 = 64;
    pub const RitualMaxBatchOffers: u32 = 16;
    pub const RitualDefaultCooldown: u32 = 0;
    pub const RitualDefaultBurnout: u32 = 0;
}

impl pallet_ritual::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RitualNsBytes = RitualNsBytes;
    type MaxSpecs = RitualMaxSpecs;
    type MaxSpecsPerKind = RitualMaxSpecsPerKind;
    type MaxSpecsPerProvider = RitualMaxSpecsPerProvider;
    type MaxMemoLen = RitualMaxMemoLen;
    type MaxCidLen = RitualMaxCidLen;
    type MaxBatchOffers = RitualMaxBatchOffers;
    type DefaultCooldownBlocks = RitualDefaultCooldown;
    type DefaultBurnoutBlocks = RitualDefaultBurnout;
    type TargetControl = ();
    type OnOfferingCommitted = ();
}

parameter_types! {
    pub const CemeteryMaxCidLen: u32 = 64;
    pub const CemeteryMaxPlotsPerCemetery: u32 = 4096;
    pub const CemeteryMaxOccupantsPerPlot: u32 = 16;
    pub const CemeteryMaxMemoLen: u32 = 64;
}
impl pallet_cemetery::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type MaxCidLen = CemeteryMaxCidLen;
    type MaxPlotsPerCemetery = CemeteryMaxPlotsPerCemetery;
    type MaxOccupantsPerPlot = CemeteryMaxOccupantsPerPlot;
    type MaxMemoLen = CemeteryMaxMemoLen;
    type OnIntermentCommitted = ();
}

parameter_types! {
    pub const DeceasedMaxCidLen: u32 = 64;
    pub const DeceasedMaxEditors: u32 = 8;
    pub const DeceasedMaxRelations: u32 = 16;
}
impl pallet_deceased::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type MaxCidLen = DeceasedMaxCidLen;
    type MaxEditors = DeceasedMaxEditors;
    type MaxRelationsPerNode = DeceasedMaxRelations;
}

parameter_types! {
    pub const OrderConfirmTTL: BlockNumber = 2 * DAYS; // 两天确认期
    pub const OrderMaxCidLen: u32 = 64;
    pub const OrderMaxImg: u32 = 20;
    pub const OrderMaxVid: u32 = 5;
    pub const OrderPlatformFeeBps: u16 = 200; // 2%
}
impl pallet_order::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type PalletIdGet = ConstPalletId;
    type PlatformAccount = PlatformAccount;
    type PlatformFeeBps = OrderPlatformFeeBps;
    type ConfirmTTL = OrderConfirmTTL;
    type MaxCidLen = OrderMaxCidLen;
    type MaxImg = OrderMaxImg;
    type MaxVid = OrderMaxVid;
    type Escrow = pallet_escrow::Pallet<Runtime>;
    // 函数级中文注释：
    // 通过本地适配 Trait（`KarmaMint`）为订单绑定 Karma 增发实现。
    // 这里将其桥接到 `pallet_karma::Pallet<Runtime>` 的 `gain` 方法。
    type Karma = KarmaMintAdapter;
    type WeightInfo = pallet_order::weights::SubstrateWeight<Runtime>;
}

// ==== Karma 适配器：桥接 pallet_karma 到 pallet_order 的本地 Trait ====
pub struct KarmaMintAdapter;
impl pallet_order::pallet::KarmaMint<AccountId> for KarmaMintAdapter {
    type Balance = Balance;
    fn gain(origin_caller: &AccountId, who: &AccountId, amount: Self::Balance, memo: Vec<u8>) -> frame_support::dispatch::DispatchResult {
        // 直接复用 Karma Pallet 的对外安全接口（含白名单校验与历史记录）
        <pallet_karma::Pallet<Runtime> as pallet_karma::pallet::KarmaCurrency<AccountId>>::gain(origin_caller, who, amount, memo)
    }
}

// 托管 PalletId 与平台账户占位（示例）
parameter_types! {
    // PalletId 仅支持 8 字节，固定使用前 8 字节常量
    pub const ConstPalletId: frame_support::PalletId = frame_support::PalletId(*b"otc/orde");
}
pub struct PlatformAccount;
impl sp_core::Get<AccountId> for PlatformAccount { fn get() -> AccountId { sp_core::crypto::AccountId32::new([0u8;32]).into() } }

// ===== otc-market/escrow/arbitration 配置 =====
parameter_types! { pub const OtcMaxNotesLen: u32 = 128; }
impl pallet_otc_market::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type MaxNotesLen = OtcMaxNotesLen;
    type Escrow = pallet_escrow::Pallet<Runtime>;
    type WeightInfo = pallet_otc_market::weights::SubstrateWeight<Runtime>;
}

parameter_types! { pub const EscrowPalletId: frame_support::PalletId = frame_support::PalletId(*b"otc/escw"); }
impl pallet_escrow::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type EscrowPalletId = EscrowPalletId;
}

parameter_types! { pub const ArbMaxEvidence: u32 = 16; pub const ArbMaxCidLen: u32 = 64; }
impl pallet_arbitration::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type MaxEvidence = ArbMaxEvidence;
    type MaxCidLen = ArbMaxCidLen;
    type Escrow = pallet_escrow::Pallet<Runtime>;
    type WeightInfo = pallet_arbitration::weights::SubstrateWeight<Runtime>;
    type Router = ArbitrationRouter;
}

parameter_types! {
	/// Karma 在授权中心的命名空间字节（建议与 PalletId 对齐）
	pub const KarmaNsBytes: [u8; 8] = *b"karma___";
}

// ===== 仲裁域路由：把仲裁请求分发到对应业务 pallet =====
pub struct ArbitrationRouter;
impl pallet_arbitration::pallet::ArbitrationRouter<AccountId> for ArbitrationRouter {
    fn can_dispute(domain: [u8; 8], who: &AccountId, id: u64) -> bool {
        match domain {
            d if d == OrderNsBytes::get() => {
                <pallet_order::Pallet<Runtime> as pallet_order::pallet::ArbitrationOrderHook<Runtime>>::can_dispute(who, id)
            }
            _ => false,
        }
    }
    fn apply_decision(domain: [u8; 8], id: u64, decision: pallet_arbitration::pallet::Decision) -> frame_support::dispatch::DispatchResult {
        match (domain, decision) {
            (d, pallet_arbitration::pallet::Decision::Release) if d == OrderNsBytes::get() => {
                <pallet_order::Pallet<Runtime> as pallet_order::pallet::ArbitrationOrderHook<Runtime>>::arbitrate_release(id)
            }
            (d, pallet_arbitration::pallet::Decision::Refund) if d == OrderNsBytes::get() => {
                <pallet_order::Pallet<Runtime> as pallet_order::pallet::ArbitrationOrderHook<Runtime>>::arbitrate_refund(id)
            }
            (d, pallet_arbitration::pallet::Decision::Partial(bps)) if d == OrderNsBytes::get() => {
                <pallet_order::Pallet<Runtime> as pallet_order::pallet::ArbitrationOrderHook<Runtime>>::arbitrate_partial(id, bps)
            }
            _ => Err(sp_runtime::DispatchError::Other("UnsupportedDomain")),
        }
    }
}

// ===== exchange 配置 =====
use frame_support::PalletId;

parameter_types! {
    pub const ExchangeBpsDenominator: u16 = 10_000;
    pub const ExchangeMaxAllocs: u32 = 64;
    pub const ExchangeMaxMemoLen: u32 = 64;
    pub const ExchangePalletId: PalletId = PalletId(*b"xchg/kma");
    /// Exchange 管理命名空间（用于分配项增删改查授权，不用于会话代付）
    pub const ExchangeNsBytes: [u8; 8] = *b"exchange";
}

impl pallet_exchange::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type PalletIdGet = ExchangePalletId;
    type BpsDenominator = ExchangeBpsDenominator;
    type MaxAllocs = ExchangeMaxAllocs;
    type MaxMemoLen = ExchangeMaxMemoLen;
    type AdminAuthorizerNs = ExchangeNsBytes;
    type Admin = ExchangeAdminAdapter;
    type Karma = pallet_karma::Pallet<Runtime>;
    type WeightInfo = (); // 基准后替换
}

// 为 Exchange 提供管理员校验适配：桥接到 pallet-authorizer
pub struct ExchangeAdminAdapter;
impl pallet_exchange::pallet::AdminAuthorizer<AccountId> for ExchangeAdminAdapter {
    fn is_admin(who: &AccountId) -> bool {
        let ns = ExchangeNsBytes::get();
        pallet_authorizer::Pallet::<Runtime>::is_authorized(pallet_authorizer::pallet::Namespace(ns), who)
    }
}