#![cfg(feature = "runtime-benchmarks")]
use super::*;
use frame_benchmarking::{v2::*, whitelisted_caller};
use frame_system::RawOrigin;

#[benchmarks]
mod benches {
    use super::*;

    #[benchmark]
    fn dispute() {
        let caller: T::AccountId = whitelisted_caller();
        let domain: [u8; 8] = *b"order___";
        let id: u64 = 1;
        let cids: Vec<BoundedVec<u8, T::MaxCidLen>> = Vec::new();
        #[extrinsic_call]
        dispute(RawOrigin::Signed(caller), domain, id, cids);
    }

    #[benchmark]
    fn arbitrate() {
        let caller: T::AccountId = whitelisted_caller();
        let domain: [u8; 8] = *b"order___";
        let id: u64 = 1;
        Disputed::<T>::insert(domain, id, ());
        let decision = Decision::Refund;
        #[extrinsic_call]
        arbitrate(RawOrigin::Signed(caller), domain, id, decision);
    }

    impl_benchmark_test_suite!(
        Pallet,
        crate::mock::new_test_ext(),
        crate::mock::Test,
    );
}


