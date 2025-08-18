#![cfg(feature = "runtime-benchmarks")]
use super::*;
use frame_benchmarking::{v2::*, whitelisted_caller};
use frame_system::RawOrigin;

#[benchmarks]
mod benches {
    use super::*;

    #[benchmark]
    fn place_order() {
        let caller: T::AccountId = whitelisted_caller();
        let price: T::Balance = 1u128.into();
        let amount: T::Balance = 1u128.into();
        let min: T::Balance = 1u128.into();
        #[extrinsic_call]
        place_order(RawOrigin::Signed(caller), Side::Sell, price, amount, min, 0);
    }

    #[benchmark]
    fn cancel_order() {
        let caller: T::AccountId = whitelisted_caller();
        let price: T::Balance = 1u128.into();
        let amount: T::Balance = 1u128.into();
        let min: T::Balance = 1u128.into();
        // place
        let _ = Pallet::<T>::place_order(RawOrigin::Signed(caller.clone()).into(), Side::Sell, price, amount, min, 0);
        let id = NextOrderId::<T>::get().saturating_sub(1);
        #[extrinsic_call]
        cancel_order(RawOrigin::Signed(caller), id);
    }

    impl_benchmark_test_suite!(
        Pallet,
        crate::mock::new_test_ext(),
        crate::mock::Test,
    );
}


