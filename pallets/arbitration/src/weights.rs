//! Pallet arbitration 权重接口与占位实现（上线前用 benchmark 生成）
use frame_support::weights::Weight;
use core::marker::PhantomData;

pub trait WeightInfo {
	/// dispute：与证据条数近似线性相关
	fn dispute(evidence_count: u32) -> Weight;
	/// arbitrate：常量开销
	fn arbitrate() -> Weight;
}

impl WeightInfo for () {
	fn dispute(_evidence_count: u32) -> Weight { Weight::from_parts(10_000, 0) }
	fn arbitrate() -> Weight { Weight::from_parts(10_000, 0) }
}

pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn dispute(_evidence_count: u32) -> Weight { Weight::from_parts(10_000, 0) }
	fn arbitrate() -> Weight { Weight::from_parts(10_000, 0) }
}


