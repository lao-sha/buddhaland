#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*, traits::Get};
use frame_system::pallet_prelude::*;
use sp_std::vec::Vec;

use pallet_karma::{KarmaProvider, KarmaBalance, MeritAction};

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// 祈福默认消耗的 Karma 值（可由 runtime 配置）
        #[pallet::constant]
        type DefaultPrayerCost: Get<KarmaBalance>;
    }

    /// 事件：用于前端订阅祈福相关的消费
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// 祈福成功：账户、消耗数量、祈福内容哈希（或摘要）
        Prayed(T::AccountId, KarmaBalance, Vec<u8>),
    }

    /// 错误类型
    #[pallet::error]
    pub enum Error<T> {
        /// 祈福内容不能为空
        EmptyPrayer,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// 祈福：从 Karma 扣除消耗并记录事件
        /// - 参数：content 为祈福内容摘要（链上不存明文，避免存储膨胀）
        /// - 行为：调用 KarmaProvider::consume_karma_for_merit 使用 MeritAction::Prayer
        /// - 事件：Prayed
        #[pallet::weight(10_000)]
        pub fn pray(origin: OriginFor<T>, content: Vec<u8>, amount: Option<KarmaBalance>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(!content.is_empty(), Error::<T>::EmptyPrayer);

            // 计算消耗值：优先使用外部传入，否则使用默认值
            let cost = amount.unwrap_or_else(|| T::DefaultPrayerCost::get());

            // 使用 karma pallet 的接口进行消费
            <pallet_karma::Pallet<T> as KarmaProvider<T::AccountId>>::consume_karma_for_merit(
                &who,
                cost,
                MeritAction::Prayer,
            ).map_err(|_| frame_support::dispatch::DispatchError::Other("ConsumeFailed"))?;

            Self::deposit_event(Event::Prayed(who, cost, content));
            Ok(().into())
        }
    }
}