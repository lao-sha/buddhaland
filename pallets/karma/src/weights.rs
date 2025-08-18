#![allow(unused_imports)]
#![cfg_attr(rustfmt, rustfmt_skip)]
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::weights::Weight;

/// 权重信息 Trait：为外部调用提供权重估算接口
pub trait WeightInfo {
	/// 添加授权账户的权重
	fn add_authorized_caller() -> Weight;
	/// 移除授权账户的权重
	fn remove_authorized_caller() -> Weight;
}

/// 默认的 Substrate 权重实现（占位实现，可用基准测试替换）
pub struct SubstrateWeight;

impl WeightInfo for SubstrateWeight {
	fn add_authorized_caller() -> Weight { Weight::from_parts(10_000, 0) }
	fn remove_authorized_caller() -> Weight { Weight::from_parts(10_000, 0) }
}


