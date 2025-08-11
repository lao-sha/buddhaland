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
	derive_impl, parameter_types, PalletId,
	traits::{ConstBool, ConstU128, ConstU32, ConstU64, ConstU8, VariantCountOf},
	weights::{
		constants::{RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND},
		IdentityFee, Weight,
	},
};
use frame_system::limits::{BlockLength, BlockWeights};
use pallet_transaction_payment::{ConstFeeMultiplier, FungibleAdapter, Multiplier};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_runtime::{traits::One, Perbill};
use sp_version::RuntimeVersion;

// Local module imports
use super::{
	AccountId, Aura, Balance, Balances, Block, BlockNumber, Hash, Nonce, PalletInfo, Runtime,
	RuntimeCall, RuntimeEvent, RuntimeFreezeReason, RuntimeHoldReason, RuntimeOrigin, RuntimeTask,
	System, EXISTENTIAL_DEPOSIT, SLOT_DURATION, VERSION,
};

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

// 在文件末尾添加karma pallet的配置
impl pallet_karma::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    /// 权重信息实现：当前使用默认实现 ()
    /// 如需使用基准测试权重，请在 pallets/karma/src/weights.rs 中提供 SubstrateWeight 并切换配置
    type WeightInfo = ();
    
    // 配置常量参数
    type BaseCheckinReward = ConstU128<1000>; // 每日签到基础奖励
    type MaxConsecutiveMultiplier = ConstU8<7>; // 最大连续签到奖励倍数
    type MinBlocksBetweenCheckins = ConstU32<14400>; // 两次签到最小区块间隔（约24小时）
    
    // 功德等级阈值配置
    type MeritLevelThresholds = MeritLevelThresholds;
    
    // 使用默认验证器（开发阶段）
    type Verifier = pallet_karma::AlwaysTrueVerifier;
    
    // 功德记录描述最大字节数
    type MaxDescriptionLen = ConstU32<256>;
}

// 定义功德等级阈值
parameter_types! {
    pub MeritLevelThresholds: &'static [u128] = &[
        0,      // 等级0: 0-999
        1000,   // 等级1: 1000-4999
        5000,   // 等级2: 5000-19999
        20000,  // 等级3: 20000-99999
        100000, // 等级4: 100000+
    ];
}

// Exchange Pallet 配置常量（账户与比例）
parameter_types! {
    // 示例账户：在开发网络中可用Alice/Bob等测试账户替代，这里保持占位，由链上配置赋值更合理
    pub const BlackholeAccountId: AccountId = AccountId::new([0u8; 32]);
    pub const TreasuryAccountId: AccountId = AccountId::new([1u8; 32]);
    pub const PaymasterAccountId: AccountId = AccountId::new([2u8; 32]);
    pub const ExchangeRateConst: u128 = 1000;     // 1 BUD => 1000 Karma
    pub const BurnBpsConst: u32 = 2000;           // 20%
    pub const TreasuryBpsConst: u32 = 7000;       // 70%
    pub const PaymasterBpsConst: u32 = 1000;      // 10%
    pub const BpsDenominatorConst: u32 = 10000;   // 基点基数
}

impl pallet_exchange::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type PaymasterAccount = PaymasterAccountId;
    type BlackholeAccount = BlackholeAccountId;
    type TreasuryAccount = TreasuryAccountId;
    type ExchangeRate = ExchangeRateConst;
    type BurnBps = BurnBpsConst;
    type TreasuryBps = TreasuryBpsConst;
    type PaymasterBps = PaymasterBpsConst;
    type BpsDenominator = BpsDenominatorConst;
}

// Paymaster Pallet 配置常量
parameter_types! {
    pub const PaymasterPalletId: PalletId = PalletId(*b"paymastr");
    pub const MaxBatchSize: u32 = 50;
    pub const MinimumDepositConst: Balance = 1000 * UNIT;
    pub const ServiceFeeRateConst: u32 = 100; // 1% 服务费，基点制
}

impl pallet_paymaster::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type Currency = Balances;
    type PalletId = PaymasterPalletId;
    type MaxBatchSize = MaxBatchSize;
    type MinimumDeposit = MinimumDepositConst;
    type ServiceFeeRate = ServiceFeeRateConst;
    type WeightInfo = ();
}

// 为 Prayer Pallet 提供运行时配置
parameter_types! {
    pub const DefaultPrayerCost: u128 = 100; // 祈福默认消耗 100 Karma
}

impl pallet_prayer::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type DefaultPrayerCost = DefaultPrayerCost;
}

// 为 Ritual Pallet 提供运行时配置
parameter_types! {
    pub const DefaultIncenseCost: u128 = 10;    // 上香默认消耗
    pub const DefaultLampCost: u128 = 20;       // 点灯默认消耗
    pub const DefaultFlowerCost: u128 = 30;     // 供花默认消耗
    pub const DefaultDonationCost: u128 = 50;   // 布施默认消耗
}

impl pallet_ritual::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type DefaultIncenseCost = DefaultIncenseCost;
    type DefaultLampCost = DefaultLampCost;
    type DefaultFlowerCost = DefaultFlowerCost;
    type DefaultDonationCost = DefaultDonationCost;
}

// Meditation Pallet 配置常量
parameter_types! {
    pub const EnableKarmaRewardConst: bool = true;                 // 开启奖励
    pub const BaseRewardPerMinuteConst: u128 = 3;                  // 每分钟基础奖励 3 Karma
    pub const MinDurationMinutesConst: u32 = 5;                    // 至少 5 分钟
    pub const MinQualityScoreConst: u8 = 60;                       // 数据质量阈值 60
}

impl pallet_meditation::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = (); // 暂用默认占位权重
    type Moment = u64;    // 与 timestamp::Config::Moment 对齐
    type EnableKarmaReward = EnableKarmaRewardConst;
    type BaseRewardPerMinute = BaseRewardPerMinuteConst;
    type MinDurationMinutes = MinDurationMinutesConst;
    type MinQualityScore = MinQualityScoreConst;
    type Karma = pallet_karma::Pallet<Runtime>;
}
