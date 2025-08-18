#![cfg(feature = "runtime-benchmarks")]
use super::*;
use frame_benchmarking::{v2::*, whitelisted_caller};
use frame_system::RawOrigin;

#[benchmarks]
mod benches {
    use super::*;

    #[benchmark]
    fn create_order() {
        let caller: T::AccountId = whitelisted_caller();
        let locked: BalanceOf<T> = 1u32.into();
        #[extrinsic_call]
        create_order(RawOrigin::Signed(caller), 1, 1, 1, locked);
    }

    #[benchmark]
    fn confirm_done_by_buyer() {
        let caller: T::AccountId = whitelisted_caller();
        let locked: BalanceOf<T> = 1u32.into();
        // create
        let id = NextOrderId::<T>::mutate(|x| { let id=*x; *x=id.saturating_add(1); id });
        Orders::<T>::insert(id, Order { buyer: caller.clone(), agent: Some(caller.clone()), temple_id: 1, service_id: 1, qty: 1, locked, status: OrderStatus::Submitted, decision_deadline: None });
        #[extrinsic_call]
        confirm_done_by_buyer(RawOrigin::Signed(caller), id);
    }

    impl_benchmark_test_suite!(
        Pallet,
        crate::mock::new_test_ext(),
        crate::mock::Test,
    );
}


