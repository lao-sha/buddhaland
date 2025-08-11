#![cfg_attr(not(feature = "std"), no_std)]

//! Meditation Pallet
//! 目标：
//! - 上链冥想(禅修)会话摘要；
//! - 基础反作弊校验钩子；
//! - 与 Karma Pallet 集成：可选地奖励 Karma；
//! - 提供近历史查询接口。

pub use pallet::*;

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

use frame_support::{
    dispatch::{DispatchResult, DispatchResultWithPostInfo},
    pallet_prelude::*,
    traits::{Get},
};
use frame_system::pallet_prelude::*;
use sp_std::vec::Vec;

/// 冥想会话标识
pub type SessionId = u64;

// 新增：RewardHook trait，供外部奖励模块实现
pub trait RewardHook<AccountId, Moment> {
    fn on_session_submitted(
        who: &AccountId,
        session_id: SessionId,
        metrics: MeditationMetrics,
        session: MeditationSession<Moment>,
    );
}

/// 冥想指标（简化版：0-100）
#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
pub struct MeditationMetrics {
    pub meditation_depth: u8,
    pub focus_level: u8,
    pub alpha_power: u8,
    pub theta_power: u8,
}

/// 冥想会话摘要（与前端/设备端约定的摘要格式）
#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
pub struct MeditationSession<Moment> {
    pub start_time: Moment,
    pub duration_minutes: u32,
    pub avg_meditation_depth: u8,
    pub avg_focus_level: u8,
    pub peak_meditation_depth: u8,
    pub brainwave_quality_score: u8,
    pub verified: bool,
}

/// 奖励原因（映射到 Karma 中的 RewardReason::Meditation）
#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
pub enum MeditationRewardReason {
    Meditation,
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use pallet_timestamp::Pallet as Timestamp;
    use pallet_karma::{KarmaProvider, RewardReason};

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// 事件类型
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// 权重信息
        type WeightInfo: WeightInfo;
        /// 与 timestamp 集成：Moment 类型
        type Moment: Parameter + Default + MaxEncodedLen + TypeInfo + Copy;
        /// 奖励开关（是否对有效会话发放 Karma）
        #[pallet::constant]
        type EnableKarmaReward: Get<bool>;
        /// 基础奖励（每分钟乘以该值）
        #[pallet::constant]
        type BaseRewardPerMinute: Get<u128>;
        /// 数据最小时长（分钟）
        #[pallet::constant]
        type MinDurationMinutes: Get<u32>;
        /// 质量阈值（0-100）
        #[pallet::constant]
        type MinQualityScore: Get<u8>;
        /// 与 Karma 集成接口
        type Karma: KarmaProvider<Self::AccountId>;
        /// 新增：会话提交后的奖励回调接口
        type Reward: RewardHook<Self::AccountId, Self::Moment>;
    }

    /// 用户 -> 下一个会话ID
    #[pallet::storage]
    #[pallet::getter(fn next_session_id)]
    pub type NextSessionId<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, SessionId, ValueQuery>;

    /// 用户 -> (session_id -> 会话数据)
    #[pallet::storage]
    #[pallet::getter(fn sessions)]
    pub type Sessions<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat, T::AccountId,
        Blake2_128Concat, SessionId,
        MeditationSession<T::Moment>,
        OptionQuery,
    >;

    /// 错误类型
    #[pallet::error]
    pub enum Error<T> {
        TooShort,
        LowQuality,
        ZeroReward,
        Overflow,
        AlreadyVerified,
    }

    /// 事件类型
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        SessionSubmitted(T::AccountId, SessionId),
        SessionVerified(T::AccountId, SessionId),
        KarmaRewarded(T::AccountId, SessionId, u128),
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// 提交冥想会话摘要（链上仅存摘要，不存原始脑波）
        /// - 验证最小时长与质量阈值
        /// - 记录会话并返回 session_id
        /// - 可选触发 Karma 奖励
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::submit_session())]
        pub fn submit_session(
            origin: OriginFor<T>,
            metrics: MeditationMetrics,
            mut session: MeditationSession<T::Moment>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            // 1) 基础阈值校验
            ensure!(session.duration_minutes >= T::MinDurationMinutes::get(), Error::<T>::TooShort);
            ensure!(session.brainwave_quality_score >= T::MinQualityScore::get(), Error::<T>::LowQuality);

            // 2) 默认把当前区块时间填充到会话（若前端未填）
            if session.start_time == Default::default() {
                // 使用 timestamp pallet 的 now 作为 Moment
                let now: T::Moment = Timestamp::<T>::now().saturated_into();
                session.start_time = now;
            }

            // 3) 分配会话ID并存储
            let next = NextSessionId::<T>::get(&who);
            let session_id = next;
            NextSessionId::<T>::insert(&who, session_id.saturating_add(1));
            Sessions::<T>::insert(&who, session_id, &session);
            Self::deposit_event(Event::SessionSubmitted(who.clone(), session_id));

            // 4) 调用外部奖励回调（由 reward pallet 实现）
            T::Reward::on_session_submitted(&who, session_id, metrics, session.clone());

            Ok(().into())
        }

        /// 标记已验证（预留：可结合链下证明或 ZK 验证）
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::verify_session())]
        pub fn verify_session(origin: OriginFor<T>, who: T::AccountId, session_id: SessionId) -> DispatchResult {
            ensure_root(origin)?;
            if let Some(mut s) = Sessions::<T>::get(&who, session_id) {
                ensure!(!s.verified, Error::<T>::AlreadyVerified);
                s.verified = true;
                Sessions::<T>::insert(&who, session_id, s);
                Self::deposit_event(Event::SessionVerified(who, session_id));
            }
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {}

    // 权重接口（占位实现）
    pub trait WeightInfo {
        fn submit_session() -> Weight;
        fn verify_session() -> Weight;
    }

    impl WeightInfo for () {
        fn submit_session() -> Weight { 10_000 }
        fn verify_session() -> Weight { 5_000 }
    }
}