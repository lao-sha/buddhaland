//! 开发期占位权重，上线前请用 benchmark 生成
use frame_support::weights::Weight;
use core::marker::PhantomData;

pub trait WeightInfo {
	fn place_order() -> Weight;
	fn cancel_order() -> Weight;
}

impl WeightInfo for () {
	fn place_order() -> Weight { Weight::from_parts(10_000, 0) }
	fn cancel_order() -> Weight { Weight::from_parts(10_000, 0) }
}

pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn place_order() -> Weight { Weight::from_parts(10_000, 0) }
	fn cancel_order() -> Weight { Weight::from_parts(10_000, 0) }
}


