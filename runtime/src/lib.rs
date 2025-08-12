#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub mod apis;
#[cfg(feature = "runtime-benchmarks")]
mod benchmarks;
pub mod configs;

extern crate alloc;
use alloc::vec::Vec;
use sp_runtime::{
	generic, impl_opaque_keys,
	traits::{BlakeTwo256, IdentifyAccount, Verify},
	MultiAddress, MultiSignature,
};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

pub use frame_system::Call as SystemCall;
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

pub mod genesis_config_presets;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;
	use sp_runtime::{
		generic,
		traits::{BlakeTwo256, Hash as HashT},
	};

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;
	/// Opaque block hash type.
	pub type Hash = <BlakeTwo256 as HashT>::Output;
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
		pub grandpa: Grandpa,
	}
}

// To learn more about runtime versioning, see:
// https://docs.substrate.io/main-docs/build/upgrade#runtime-versioning
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: alloc::borrow::Cow::Borrowed("solochain-template-runtime"),
	impl_name: alloc::borrow::Cow::Borrowed("solochain-template-runtime"),
	authoring_version: 1,
	// The version of the runtime specification. A full node will not attempt to use its native
	//   runtime in substitute for the on-chain Wasm runtime unless all of `spec_name`,
	//   `spec_version`, and `authoring_version` are the same between Wasm and native.
	// This value is set to 100 to notify Polkadot-JS App (https://polkadot.js.org/apps) to use
	//   the compatible custom types.
	spec_version: 100,
	impl_version: 1,
	apis: apis::RUNTIME_API_VERSIONS,
	transaction_version: 1,
	system_version: 1,
};

mod block_times {
	/// This determines the average expected block time that we are targeting. Blocks will be
	/// produced at a minimum duration defined by `SLOT_DURATION`. `SLOT_DURATION` is picked up by
	/// `pallet_timestamp` which is in turn picked up by `pallet_aura` to implement `fn
	/// slot_duration()`.
	///
	/// Change this to adjust the block time.
	pub const MILLI_SECS_PER_BLOCK: u64 = 6000;

	// NOTE: Currently it is not possible to change the slot duration after the chain has started.
	// Attempting to do so will brick block production.
	pub const SLOT_DURATION: u64 = MILLI_SECS_PER_BLOCK;
}
pub use block_times::*;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLI_SECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

pub const BLOCK_HASH_COUNT: BlockNumber = 2400;

// Unit = the base number of indivisible units for balances
pub const UNIT: Balance = 1_000_000_000_000;
pub const MILLI_UNIT: Balance = 1_000_000_000;
pub const MICRO_UNIT: Balance = 1_000_000;

/// Existential deposit.
pub const EXISTENTIAL_DEPOSIT: Balance = MILLI_UNIT;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// An index to a block.
pub type BlockNumber = u32;

/// The address format for describing accounts.
pub type Address = MultiAddress<AccountId, ()>;

/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;

/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;

/// The `TransactionExtension` to the basic transaction logic.
pub type TxExtension = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	frame_metadata_hash_extension::CheckMetadataHash<Runtime>,
	frame_system::WeightReclaim<Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;

/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, TxExtension>;

/// All migrations of the runtime, aside from the ones declared in the pallets.
///
/// This can be a tuple of types, each implementing `OnRuntimeUpgrade`.
#[allow(unused_parens)]
type Migrations = ();

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
	Migrations,
>;

// Create the runtime by composing the FRAME pallets that were previously configured.
#[frame_support::runtime]
mod runtime {
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask,
		RuntimeViewFunction
	)]
	pub struct Runtime;

	#[runtime::pallet_index(0)]
	pub type System = frame_system;

	#[runtime::pallet_index(1)]
	pub type Timestamp = pallet_timestamp;

	#[runtime::pallet_index(2)]
	pub type Aura = pallet_aura;

	#[runtime::pallet_index(3)]
	pub type Grandpa = pallet_grandpa;

	#[runtime::pallet_index(4)]
	pub type Balances = pallet_balances;

	#[runtime::pallet_index(5)]
	pub type TransactionPayment = pallet_transaction_payment;

	#[runtime::pallet_index(6)]
	pub type Sudo = pallet_sudo;

	// Include the custom logic from the pallet-template in the runtime.
	#[runtime::pallet_index(7)]
	pub type Template = pallet_template;

	// Include the karma pallet in the runtime.
	#[runtime::pallet_index(8)]
	pub type Karma = pallet_karma;

	// Include the prayer pallet in the runtime.
	#[runtime::pallet_index(9)]
	pub type Prayer = pallet_prayer;

	// Include the ritual pallet in the runtime.
	#[runtime::pallet_index(10)]
	pub type Ritual = pallet_ritual;

	// Include the exchange pallet in the runtime.
	#[runtime::pallet_index(11)]
	pub type Exchange = pallet_exchange;

	// Include the meditation pallet in the runtime.
	#[runtime::pallet_index(12)]
	pub type Meditation = pallet_meditation;

	// Include the paymaster pallet in the runtime.
	#[runtime::pallet_index(13)]
	pub type Paymaster = pallet_paymaster;

	// Include the share-mining pallet in the runtime.
	#[runtime::pallet_index(14)]
	pub type ShareMining = pallet_share_mining;

	// Include the commemorate pallet in the runtime.
	#[runtime::pallet_index(10)]
	pub type Commemorate = pallet_commemorate;
}

use codec::{Encode, Decode};
use frame_support::traits::tokens::imbalance::Imbalance;
use sp_runtime::{
    transaction_validity::{ValidTransaction, InvalidTransaction, TransactionValidityError},
    traits::DispatchInfoOf,
};
use sp_std::marker::PhantomData;

/// 自动代付交易扩展（示例）
/// 目的：在交易进入池前检查是否满足免签代付条件，并在 prepare 阶段进行必要的准备工作
#[derive(Encode, Decode, Clone, Eq, PartialEq, scale_info::TypeInfo)]
pub struct AutoPayTxExtension<Runtime>(PhantomData<Runtime>);

impl<Runtime> sp_runtime::TransactionExtension<Runtime::RuntimeCall> for AutoPayTxExtension<Runtime>
where
    Runtime: frame_system::Config + pallet_paymaster::Config,
{
    type Implicit = ();     // 可根据需要放置隐式参数
    type Val = ();          // validate 阶段产物
    type Pre = ();          // prepare 阶段产物

    /// 交易池校验阶段（off-chain/on-chain皆会调用，不可变）
    /// - 检查调用是否为可免签代付的范围（白名单）
    /// - 检查赞助方是否预授权且未过期
    /// - 不在此处做状态写入
    fn validate(
        &self,
        who: &<Runtime as frame_system::Config>::AccountId,
        call: &Runtime::RuntimeCall,
        info: &DispatchInfoOf<Runtime::RuntimeCall>,
        _len: usize,
        _implicit: &Self::Implicit,
        _ctx: &mut sp_runtime::transaction_validity::Context,
    ) -> sp_runtime::TransactionValidity {
        // 示例：仅当调用属于特定模块且用户具备资格时，才标记有效
        // 实际中可解构 call，判断是否需要自动代付
        let _ = (who, call, info);
        Ok(ValidTransaction::default())
    }

    /// 区块执行前的准备阶段（on-chain，可变）
    /// - 可进行系统池余额预检查、预保留
    /// - 也可登记“本次交易启用免签代付”的上下文，供 call 执行阶段读取
    fn prepare(
        self,
        _val: Self::Val,
        _who: &<Runtime as frame_system::Config>::AccountId,
        _call: &Runtime::RuntimeCall,
        _info: &DispatchInfoOf<Runtime::RuntimeCall>,
        _len: usize,
        _implicit: Self::Implicit,
    ) -> Result<Self::Pre, TransactionValidityError> {
        Ok(())
    }
}

// 将扩展接入 runtime（示例）
type RuntimeTransactionExtensions = (
    sp_runtime::AsTransactionExtension<AutoPayTxExtension<Runtime>>, // 适配器：兼容旧接口的写法
    // 其他扩展...
);
