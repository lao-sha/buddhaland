#![allow(unused_imports)]
use frame_support::weights::Weight;

/// 权重接口（占位版，后续用基准生成替换）
pub trait WeightInfo {
	/// 批量设置分配项，n为分配项数量
	fn set_allocs(n: u32) -> Weight;
	/// 兑换，n为当前启用分配项数量
	fn exchange(n: u32) -> Weight;
}

impl WeightInfo for () {
	fn set_allocs(_n: u32) -> Weight { Weight::from_parts(10_000, 0) }
	fn exchange(_n: u32) -> Weight { Weight::from_parts(10_000, 0) }
}


