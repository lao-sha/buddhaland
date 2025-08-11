#![cfg_attr(not(feature = "std"), no_std)]

//! Reward Pallet
//! 职责：监听 meditation pallet 的冥想会话提交事件，
//! 根据会话摘要与指标计算 Karma 奖励，并调用 karma pallet 发放。

pub use pallet::*;

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

use frame_support::{
    dispatch::DispatchResult,
    pallet_prelude::*,
    traits::Get,
};
use frame_system::pallet_prelude::*;
use sp_std::vec::Vec;

/// 冥想奖励配置项：用于权重调优
#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
pub struct RewardWeights {
    /// 每分钟基础奖励（与 meditation 的 BaseRewardPerMinute 区分，这里是 reward pallet 层的附加倍率或基数）
    pub base_per_minute: u128,
    /// 深度加成权重
    pub depth_weight: u8,
    /// 专注加成权重
    pub focus_weight: u8,
    /// 质量分加成权重
    pub quality_weight: u8,
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use pallet_karma::{KarmaProvider, RewardReason};
    use pallet_meditation::{MeditationMetrics, MeditationSession, SessionId};

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type WeightInfo: WeightInfo;
        /// 奖励权重配置
        #[pallet::constant]
        type RewardWeights: Get<RewardWeights>;
        /// 是否仅奖励通过验证的会话
        #[pallet::constant]
        type OnlyRewardVerified: Get<bool>;
        /// Karma 提供者
        type Karma: KarmaProvider<Self::AccountId>;
        /// meditation pallet 中 Moment 类型（用于事件携带/兼容）
        type Moment: Parameter + Default + MaxEncodedLen + TypeInfo + Copy;
    }

    /// 事件
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// 已根据冥想会话发放奖励 (who, session_id, amount)
        RewardGranted(T::AccountId, SessionId, u128),
    }

    /// 错误
    #[pallet::error]
    pub enum Error<T> {
        ZeroReward,
        Overflow,
        NotVerified,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// 手动触发对某个冥想会话的奖励计算与发放（例如管理员或自动化任务调用）
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::grant_for_session())]
        pub fn grant_for_session(
            origin: OriginFor<T>,
            who: T::AccountId,
            session_id: SessionId,
            metrics: MeditationMetrics,
            session: MeditationSession<T::Moment>,
        ) -> DispatchResult {
            ensure_root(origin)?; // 管理/系统调用

            let amount = Self::calculate_reward_internal(&metrics, &session);
            ensure!(amount > 0, Error::<T>::ZeroReward);

            if T::OnlyRewardVerified::get() {
                ensure!(session.verified, Error::<T>::NotVerified);
            }

            T::Karma::reward_karma(&who, amount, RewardReason::Meditation)
                .map_err(|_| Error::<T>::Overflow)?;
            Self::deposit_event(Event::RewardGranted(who, session_id, amount));
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// 内部奖励计算：示例线性组合
        pub fn calculate_reward_internal(
            metrics: &MeditationMetrics,
            session: &MeditationSession<T::Moment>,
        ) -> u128 {
            let w = T::RewardWeights::get();
            let base = (session.duration_minutes as u128).saturating_mul(w.base_per_minute);
            let depth = (metrics.meditation_depth as u128).saturating_mul(w.depth_weight as u128);
            let focus = (metrics.focus_level as u128).saturating_mul(w.focus_weight as u128);
            let quality = (session.brainwave_quality_score as u128).saturating_mul(w.quality_weight as u128);
            base.saturating_add(depth).saturating_add(focus).saturating_add(quality)
        }
    }

    /// 实现 RewardHook trait，用于 meditation pallet 回调
    impl<T: Config> pallet_meditation::RewardHook<T::AccountId, T::Moment> for Pallet<T> {
        fn on_session_submitted(
            who: &T::AccountId,
            session_id: SessionId,
            metrics: MeditationMetrics,
            session: MeditationSession<T::Moment>,
        ) {
            // 检查是否仅奖励已验证会话
            if T::OnlyRewardVerified::get() && !session.verified {
                return;
            }

            let amount = Self::calculate_reward_internal(&metrics, &session);
            if amount > 0 {
                if let Ok(()) = T::Karma::reward_karma(who, amount, RewardReason::Meditation) {
                    Self::deposit_event(Event::RewardGranted(who.clone(), session_id, amount));
                }
                // 错误情况下静默处理，避免影响 meditation pallet 的主流程
            }
        }
    }

    // 权重接口（占位实现）
    pub trait WeightInfo {
        fn grant_for_session() -> Weight;
    }
    impl WeightInfo for () {
        fn grant_for_session() -> Weight { 10_000 }
    }
}