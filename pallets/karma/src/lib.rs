#![cfg_attr(not(feature = "std"), no_std)]

//! Karma Pallet
//! - 定位：不可转移的修为/声誉积分系统
//! - 功能：签到/任务/禅修奖励、功德行为消费、总功德值与等级、历史记录
//! - 对外：提供 KarmaProvider；任务校验通过 Verifier 抽象
//! - 安全：不可转移、限频/去重、防零值、溢出保护

pub use pallet::*;

pub mod weights;
use weights::WeightInfo;

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

use frame_support::{
    dispatch::{DispatchResult, DispatchResultWithPostInfo},
    pallet_prelude::*,
    traits::Get,
};
use frame_system::pallet_prelude::*;
use sp_runtime::traits::{Saturating, Zero};
use sp_std::vec::Vec;

/// 任务标识类型
pub type TaskId = u64;
/// Karma 余额与功德值类型
pub type KarmaBalance = u128;

/// 功德行为类型
#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
pub enum MeritAction {
    Prayer,
    Incense,
    LightLamp,
    Flower,
    Donation,
    Other(u8),
}

/// 奖励原因
#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
pub enum RewardReason {
    DailyCheckin,
    Meditation,
    TaskCompleted,
    CommunityContribution,
    ManualAdjust,
}

/// 功德消费记录
#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
pub struct MeritRecord<AccountId, BlockNumber> {
    pub id: u64,
    pub who: AccountId,
    pub action: MeritAction,
    pub amount: KarmaBalance,
    pub description: Vec<u8>,
    pub at_block: BlockNumber,
}

/// 任务完成记录
#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
pub struct TaskCompletion<BlockNumber> {
    pub task_id: TaskId,
    pub at_block: BlockNumber,
}

/// 任务/禅修 证明验证接口（通过 Config 注入具体实现）
pub trait MeditationVerifier<AccountId> {
    /// 校验任务或禅修证明
    fn verify(_who: &AccountId, _task_id: TaskId, _proof: Vec<u8>) -> bool {
        true
    }
}

/// 开发阶段默认验证器（总是通过，生产请替换）
pub struct AlwaysTrueVerifier;
impl<AccountId> MeditationVerifier<AccountId> for AlwaysTrueVerifier {}

/// 对外提供的 Karma 查询与操作接口（供其他 Pallet 调用）
pub trait KarmaProvider<AccountId> {
    /// 获取用户 Karma 余额
    fn karma_balance(who: &AccountId) -> KarmaBalance;
    /// 奖励 Karma（无需签名）
    fn reward_karma(who: &AccountId, amount: KarmaBalance, reason: RewardReason) -> DispatchResult;
    /// 消费 Karma 用于功德（无需签名）
    fn consume_karma_for_merit(
        who: &AccountId,
        amount: KarmaBalance,
        action: MeritAction,
    ) -> DispatchResult;
    /// 获取用户总功德值
    fn total_merit_value(who: &AccountId) -> KarmaBalance;
    /// 获取用户修为等级
    fn merit_level(who: &AccountId) -> u8;
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// Pallet 配置
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// 运行时事件
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// 权重信息
        type WeightInfo: WeightInfo;
        /// 每日签到基础奖励
        #[pallet::constant]
        type BaseCheckinReward: Get<KarmaBalance>;
        /// 最大连续签到奖励倍数
        #[pallet::constant]
        type MaxConsecutiveMultiplier: Get<u8>;
        /// 两次签到最小区块间隔（防刷）
        #[pallet::constant]
        type MinBlocksBetweenCheckins: Get<Self::BlockNumber>;
        /// 功德等级阈值（按总功德值划分）
        type MeritLevelThresholds: Get<&'static [KarmaBalance]>;
        /// 任务/禅修证明验证器
        type Verifier: MeditationVerifier<Self::AccountId>;
    }

    /// 用户 Karma 余额（不可转移）
    #[pallet::storage]
    #[pallet::getter(fn karma_balances)]
    pub type KarmaBalances<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, KarmaBalance, ValueQuery>;

    /// 用户总功德值（累计消费总额，永久记录）
    #[pallet::storage]
    #[pallet::getter(fn total_merit_value)]
    pub type TotalMeritValue<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, KarmaBalance, ValueQuery>;

    /// 用户修为等级（由总功德值推导）
    #[pallet::storage]
    #[pallet::getter(fn user_merit_level)]
    pub type UserMeritLevel<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, u8, ValueQuery>;

    /// 每日签到记录（最近一次签到的区块号）
    #[pallet::storage]
    #[pallet::getter(fn last_checkin_block)]
    pub type DailyCheckins<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, T::BlockNumber, OptionQuery>;

    /// 每个账户的下一条功德记录 ID（自增）
    #[pallet::storage]
    #[pallet::getter(fn next_merit_record_id)]
    pub type NextMeritRecordId<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

    /// 功德消费历史记录（AccountId -> RecordId -> Record）
    #[pallet::storage]
    #[pallet::getter(fn merit_consumption_history)]
    pub type MeritConsumptionHistory<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        u64,
        MeritRecord<T::AccountId, T::BlockNumber>,
        OptionQuery,
    >;

    /// 修行任务完成记录（AccountId, TaskId -> 完成记录）
    #[pallet::storage]
    #[pallet::getter(fn completed_tasks)]
    pub type CompletedTasks<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        TaskId,
        TaskCompletion<T::BlockNumber>,
        OptionQuery,
    >;

    /// 错误类型
    #[pallet::error]
    pub enum Error<T> {
        InsufficientKarma,
        KarmaTransferNotAllowed,
        InvalidMeritAction,
        AlreadyCheckedIn,
        TaskAlreadyCompleted,
        InvalidTaskProof,
        Overflow,
        ZeroAmount,
        TooFrequent,
    }

    /// 事件类型
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        KarmaRewarded(T::AccountId, KarmaBalance, RewardReason),
        KarmaConsumed(T::AccountId, KarmaBalance, MeritAction),
        MeritActionPerformed(T::AccountId, MeritAction, Vec<u8>),
        MeritValueUpdated(T::AccountId, KarmaBalance, u8),
        DailyCheckin(T::AccountId, KarmaBalance, u32),
        TaskCompleted(T::AccountId, TaskId, KarmaBalance),
        LevelUp(T::AccountId, u8, KarmaBalance),
    }

    /// 创世配置
    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub initial_karma: Vec<(T::AccountId, KarmaBalance)>,
        pub initial_merit: Vec<(T::AccountId, KarmaBalance)>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                initial_karma: vec![],
                initial_merit: vec![],
            }
        }
    }

    /// 创世构建：初始化 Karma、总功德与等级
    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            for (account, karma) in &self.initial_karma {
                KarmaBalances::<T>::insert(account, karma);
            }
            for (account, merit) in &self.initial_merit {
                TotalMeritValue::<T>::insert(account, merit);
                let lvl = Pallet::<T>::calculate_merit_level_internal(*merit);
                UserMeritLevel::<T>::insert(account, lvl);
            }
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// 每日签到：按基础奖励与连续签到倍数发放 Karma
        /// - 防刷：与最近签到区块比较，需超过 MinBlocksBetweenCheckins
        /// - 事件：DailyCheckin、KarmaRewarded
        #[pallet::weight(T::WeightInfo::daily_checkin())]
        pub fn daily_checkin(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            let now_block = <frame_system::Pallet<T>>::block_number();
            if let Some(last) = DailyCheckins::<T>::get(&who) {
                let passed = now_block.saturating_sub(last);
                ensure!(
                    passed >= T::MinBlocksBetweenCheckins::get(),
                    Error::<T>::AlreadyCheckedIn
                );
            }

            // 简化：连续签到天数（示例固定 1，可扩展为连续签到存储）
            let consecutive_days: u32 = 1;

            let multiplier =
                core::cmp::min(1u8 + (consecutive_days as u8 - 1), T::MaxConsecutiveMultiplier::get());
            let base = T::BaseCheckinReward::get();
            let reward = base.saturating_mul(multiplier as KarmaBalance);
            ensure!(!reward.is_zero(), Error::<T>::ZeroAmount);

            KarmaBalances::<T>::mutate(&who, |b| *b = b.saturating_add(reward));
            DailyCheckins::<T>::insert(&who, now_block);

            Self::deposit_event(Event::KarmaRewarded(who.clone(), reward, RewardReason::DailyCheckin));
            Self::deposit_event(Event::DailyCheckin(who, reward, consecutive_days));
            Ok(().into())
        }

        /// 完成修行任务：校验证明并发放 Karma 奖励
        /// - 防重复：同一 TaskId 仅能完成一次
        /// - 验证：通过 Config::Verifier 执行证明校验
        /// - 事件：TaskCompleted、KarmaRewarded
        #[pallet::weight(T::WeightInfo::complete_meditation_task())]
        pub fn complete_meditation_task(
            origin: OriginFor<T>,
            task_id: TaskId,
            proof: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(CompletedTasks::<T>::get(&who, task_id).is_none(), Error::<T>::TaskAlreadyCompleted);

            ensure!(T::Verifier::verify(&who, task_id, proof), Error::<T>::InvalidTaskProof);

            let reward = T::BaseCheckinReward::get();
            ensure!(!reward.is_zero(), Error::<T>::ZeroAmount);

            KarmaBalances::<T>::mutate(&who, |b| *b = b.saturating_add(reward));
            let now_block = <frame_system::Pallet<T>>::block_number();
            CompletedTasks::<T>::insert(&who, task_id, TaskCompletion { task_id, at_block: now_block });

            Self::deposit_event(Event::KarmaRewarded(who.clone(), reward, RewardReason::TaskCompleted));
            Self::deposit_event(Event::TaskCompleted(who, task_id, reward));
            Ok(().into())
        }

        /// 执行功德行为：从 Karma 余额中扣减并累计至总功德值、更新等级、记录历史
        /// - 输入：行为类型、消费数量、描述
        /// - 事件：KarmaConsumed、MeritActionPerformed、MeritValueUpdated、LevelUp（如升级）
        #[pallet::weight(T::WeightInfo::perform_merit_action())]
        pub fn perform_merit_action(
            origin: OriginFor<T>,
            action: MeritAction,
            karma_amount: KarmaBalance,
            description: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(karma_amount > 0, Error::<T>::ZeroAmount);

            KarmaBalances::<T>::try_mutate(&who, |b| -> Result<(), Error<T>> {
                ensure!(*b >= karma_amount, Error::<T>::InsufficientKarma);
                *b = b.saturating_sub(karma_amount);
                Ok(())
            })?;

            let prev_merit = TotalMeritValue::<T>::get(&who);
            let new_merit = prev_merit.saturating_add(karma_amount);
            TotalMeritValue::<T>::insert(&who, new_merit);

            let prev_level = UserMeritLevel::<T>::get(&who);
            let new_level = Self::calculate_merit_level_internal(new_merit);
            UserMeritLevel::<T>::insert(&who, new_level);

            let rec_id = NextMeritRecordId::<T>::mutate(&who, |id| {
                let curr = *id;
                *id = id.saturating_add(1);
                curr
            });
            let now_block = <frame_system::Pallet<T>>::block_number();
            let rec = MeritRecord::<T::AccountId, T::BlockNumber> {
                id: rec_id,
                who: who.clone(),
                action,
                amount: karma_amount,
                description: description.clone(),
                at_block: now_block,
            };
            MeritConsumptionHistory::<T>::insert(&who, rec_id, rec);

            Self::deposit_event(Event::KarmaConsumed(who.clone(), karma_amount, action));
            Self::deposit_event(Event::MeritActionPerformed(who.clone(), action, description));
            Self::deposit_event(Event::MeritValueUpdated(who.clone(), new_merit, new_level));
            if new_level > prev_level {
                Self::deposit_event(Event::LevelUp(who, new_level, new_merit));
            }

            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// 依据总功德值计算修为等级（内部函数）
        /// - 遍历阈值数组，返回满足的最大等级（下标+1）
        pub fn calculate_merit_level_internal(total_merit: KarmaBalance) -> u8 {
            let mut level: u8 = 0;
            for &threshold in T::MeritLevelThresholds::get().iter() {
                if total_merit >= threshold {
                    level = level.saturating_add(1);
                } else {
                    break;
                }
            }
            level
        }

        /// 查询用户最近 N 条功德记录（从高 ID 向后）
        /// - 注意：链上迭代为示例，生产建议链下索引
        pub fn get_recent_merit_records(
            who: &T::AccountId,
            count: u32,
        ) -> Vec<MeritRecord<T::AccountId, T::BlockNumber>> {
            let mut res = Vec::new();
            let mut left = count;
            let mut id = NextMeritRecordId::<T>::get(who);
            while left > 0 && id > 0 {
                id -= 1;
                if let Some(r) = MeritConsumptionHistory::<T>::get(who, id) {
                    res.push(r);
                    left -= 1;
                } else {
                    break;
                }
            }
            res
        }

        /// 系统内部：奖励 Karma（无签名调用）
        /// - 用于任务、禅修、贡献激励等
        pub fn do_reward_karma(who: &T::AccountId, amount: KarmaBalance, reason: RewardReason) -> DispatchResult {
            ensure!(amount > 0, Error::<T>::ZeroAmount);
            KarmaBalances::<T>::mutate(who, |b| *b = b.saturating_add(amount));
            Self::deposit_event(Event::KarmaRewarded(who.clone(), amount, reason));
            Ok(())
        }

        /// 系统内部：消费 Karma 用于功德（无签名调用）
        /// - 用于系统触发的捐赠、祈福等
        pub fn do_consume_karma_for_merit(
            who: &T::AccountId,
            amount: KarmaBalance,
            action: MeritAction,
            description: Vec<u8>,
        ) -> DispatchResult {
            ensure!(amount > 0, Error::<T>::ZeroAmount);

            KarmaBalances::<T>::try_mutate(who, |b| -> Result<(), Error<T>> {
                ensure!(*b >= amount, Error::<T>::InsufficientKarma);
                *b = b.saturating_sub(amount);
                Ok(())
            })?;

            let prev_merit = TotalMeritValue::<T>::get(who);
            let new_merit = prev_merit.saturating_add(amount);
            TotalMeritValue::<T>::insert(who, new_merit);

            let prev_level = UserMeritLevel::<T>::get(who);
            let new_level = Self::calculate_merit_level_internal(new_merit);
            UserMeritLevel::<T>::insert(who, new_level);

            let rec_id = NextMeritRecordId::<T>::mutate(who, |id| {
                let curr = *id;
                *id = id.saturating_add(1);
                curr
            });
            let now_block = <frame_system::Pallet<T>>::block_number();
            let rec = MeritRecord::<T::AccountId, T::BlockNumber> {
                id: rec_id,
                who: who.clone(),
                action,
                amount,
                description: description.clone(),
                at_block: now_block,
            };
            MeritConsumptionHistory::<T>::insert(who, rec_id, rec);

            Self::deposit_event(Event::KarmaConsumed(who.clone(), amount, action));
            Self::deposit_event(Event::MeritActionPerformed(who.clone(), action, description));
            Self::deposit_event(Event::MeritValueUpdated(who.clone(), new_merit, new_level));
            if new_level > prev_level {
                Self::deposit_event(Event::LevelUp(who.clone(), new_level, new_merit));
            }

            Ok(())
        }
    }

    /// 为 Pallet 实现 KarmaProvider 接口，供外部调用
    impl<T: Config> super::KarmaProvider<T::AccountId> for Pallet<T> {
        fn karma_balance(who: &T::AccountId) -> KarmaBalance {
            KarmaBalances::<T>::get(who)
        }
        fn reward_karma(who: &T::AccountId, amount: KarmaBalance, reason: RewardReason) -> DispatchResult {
            Self::do_reward_karma(who, amount, reason)
        }
        fn consume_karma_for_merit(who: &T::AccountId, amount: KarmaBalance, action: MeritAction) -> DispatchResult {
            Self::do_consume_karma_for_merit(who, amount, action, Vec::new())
        }
        fn total_merit_value(who: &T::AccountId) -> KarmaBalance {
            TotalMeritValue::<T>::get(who)
        }
        fn merit_level(who: &T::AccountId) -> u8 {
            UserMeritLevel::<T>::get(who)
        }
    }
}