//! 开发期占位权重，上线前请用 benchmark 生成
use frame_support::weights::Weight;
use frame_support::traits::Get;
use core::marker::PhantomData;

pub trait WeightInfo {
    fn create_order() -> Weight;
    fn accept_order() -> Weight;
    fn start_order() -> Weight;
    fn submit_order_proof(count: u32) -> Weight;
    fn confirm_done_by_buyer() -> Weight;
    fn finalize_expired() -> Weight;
}

impl WeightInfo for () {
    fn create_order() -> Weight { Weight::from_parts(10_000, 0) }
    fn accept_order() -> Weight { Weight::from_parts(10_000, 0) }
    fn start_order() -> Weight { Weight::from_parts(10_000, 0) }
    fn submit_order_proof(_count: u32) -> Weight { Weight::from_parts(20_000, 0) }
    fn confirm_done_by_buyer() -> Weight { Weight::from_parts(10_000, 0) }
    fn finalize_expired() -> Weight { Weight::from_parts(10_000, 0) }
}

// 基于 frame-benchmarking 自动生成的权重结构占位（运行基准后可替换具体数值）
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn create_order() -> Weight { Weight::from_parts(10_000, 0) }
    fn accept_order() -> Weight { Weight::from_parts(10_000, 0) }
    fn start_order() -> Weight { Weight::from_parts(10_000, 0) }
    fn submit_order_proof(_count: u32) -> Weight { Weight::from_parts(20_000, 0) }
    fn confirm_done_by_buyer() -> Weight { Weight::from_parts(10_000, 0) }
    fn finalize_expired() -> Weight { Weight::from_parts(10_000, 0) }
}


