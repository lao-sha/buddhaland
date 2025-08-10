# Buddha Karma Pallet

佛境项目的修为/声誉积分系统，管理用户的 Karma（业力/福缘值）作为不可转移的个人修行与功德累积指标。

Karma 是一种非经济化的、不可转移的声誉积分，用于反映用户在佛境中的修行成长、功德行为与社区贡献。它不支持转账或交易，也不与任何代币兑换挂钩；系统通过签到、修行、功德行为等方式发放 Karma，并在功德行为中消费 Karma，同步累积“总功德值”与“修为等级”。

---

## 功能概述

- 不可转移的声誉积分（Karma）：绑定账户，强调个人修行与功德的长期积累
- 修行与功德闭环：通过签到/任务/禅修等获得 Karma，在功德行为中消费 Karma，并记录为长期功德值
- 永久记录：“总功德值”永久记录，修为等级随之动态更新
- 系统/跨 Pallet 友好：提供外部 Trait 接口，支持其他 Pallet（如禅修与奖励）调用与集成

---

## 目标与非目标

- 目标
  - 作为修为/声誉系统，为用户提供长期修行与功德成长的正反馈
  - 通过透明可验证的记录，支撑等级、成就与社区声誉体系
  - 提供跨 Pallet 的通用接口，便于任务、奖励、治理等模块集成

- 非目标（明确排除）
  - ❌ 不作为可交易代币使用
  - ❌ 不支持任何形式的转账与兑换
  - ❌ 不与法币或其他加密资产建立兑价关系
  - ❌ 不提供二级市场或交易功能

---

## 功能特性

1. Karma 获取
   - 每日签到（防刷机制、连续签到奖励加成）
   - 修行任务（任务验证，按规则发放奖励）
   - 禅修行为（基于时长/质量的加权奖励）
   - 社区贡献（如内容创作、互助等）

2. Karma 消费与功德
   - 功德行为：祈福、上香、点灯、献花、捐赠等
   - 自动消费：可由系统或其他 Pallet 无需签名触发
   - 消费即记录：消费金额累计入“总功德值”，并用于“修为等级”计算

3. 修为等级系统
   - 基于总功德值跨越阈值动态计算
   - 功德消费时自动更新等级
   - 不同等级可解锁不同修行权益或成就

4. 历史记录与统计
   - 记录每次功德行为（类型、时间、数量、备注）
   - 可查询个人总功德值、等级、消费历史、成长趋势

---

## 设计原则

- 个人修行优先：以精神成长与自我提升为核心，不经济化
- 不可转移：Karma 严格绑定账户，杜绝投机与滥用
- 透明与可验证：所有奖励与消费均可审计
- 易于集成：通过 Trait 对外暴露能力，便于多 Pallet 协同

---

## 存储设计

```rust
/// 用户 Karma 余额（不可转移）
pub type KarmaBalances<T> = StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

/// 用户总功德值（累计消费总额，永久记录）
pub type TotalMeritValue<T> = StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

/// 用户修为等级（由总功德值推导）
pub type UserMeritLevel<T> = StorageMap<_, Blake2_128Concat, T::AccountId, u8, ValueQuery>;

/// 每日签到记录（保存最近一次签到区块/时间）
pub type DailyCheckins<T> = StorageMap<_, Blake2_128Concat, T::AccountId, T::BlockNumber, OptionQuery>;

/// 功德消费历史记录（递增 ID -> 记录结构）
pub type MeritConsumptionHistory<T> =
    StorageDoubleMap<_, Blake2_128Concat, T::AccountId, Blake2_128Concat, u64, MeritRecord<T>, OptionQuery>;

/// 修行任务完成记录（按账户与任务 ID 记录）
pub type CompletedTasks<T> =
    StorageDoubleMap<_, Blake2_128Concat, T::AccountId, Blake2_128Concat, TaskId, TaskCompletion<T>, OptionQuery>;
```

---

## 配置项（Config）

```rust
/// 每日签到基础奖励
#[pallet::constant]
type BaseCheckinReward: Get<u64>;

/// 最大连续签到奖励倍数
#[pallet::constant]
type MaxConsecutiveMultiplier: Get<u8>;

/// 功德等级阈值（如 [1_000, 10_000, 100_000, ...]）
#[pallet::constant]
type MeritLevelThresholds: Get<Vec<u64>>;

/// 时间提供者（用于防刷、限频）
type TimeProvider: UnixTime;

/// PalletId（若需事件标识/账户派生）
#[pallet::constant]
type PalletId: Get<PalletId>;
```

---

## 错误类型

```rust
#[pallet::error]
pub enum Error<T> {
    /// Karma 余额不足
    InsufficientKarma,
    /// Karma 不允许转账
    KarmaTransferNotAllowed,
    /// 无效的功德行为
    InvalidMeritAction,
    /// 今日已签到
    AlreadyCheckedIn,
    /// 任务已完成
    TaskAlreadyCompleted,
    /// 无效的任务证明
    InvalidTaskProof,
    /// 数值溢出
    Overflow,
    /// 数量为零
    ZeroAmount,
    /// 操作过于频繁（限频/防刷）
    TooFrequent,
}
```

---

## 事件类型

```rust
#[pallet::event]
#[pallet::generate_deposit(pub(super) fn deposit_event)]
pub enum Event<T: Config> {
    /// Karma 奖励发放 [账户, 数量, 原因]
    KarmaRewarded(T::AccountId, u64, RewardReason),
    /// Karma 消费用于功德 [账户, 数量, 行为类型]
    KarmaConsumed(T::AccountId, u64, MeritAction),
    /// 功德行为执行 [账户, 行为类型, 描述]
    MeritActionPerformed(T::AccountId, MeritAction, Vec<u8>),
    /// 功德值更新 [账户, 新总功德值, 等级]
    MeritValueUpdated(T::AccountId, u64, u8),
    /// 每日签到 [账户, 奖励数量, 连续天数]
    DailyCheckin(T::AccountId, u64, u32),
    /// 任务完成 [账户, 任务ID, 奖励数量]
    TaskCompleted(T::AccountId, TaskId, u64),
    /// 等级提升 [账户, 新等级, 总功德值]
    LevelUp(T::AccountId, u8, u64),
}
```

---

## 外部接口（Trait，跨 Pallet 解耦）

```rust
/// Karma 提供者接口：为其他 Pallet 提供查询与读写能力
pub trait KarmaProvider<AccountId> {
    /// 获取用户 Karma 余额
    fn karma_balance(who: &AccountId) -> u64;

    /// 奖励 Karma（系统/其他 Pallet 调用）
    fn reward_karma(who: &AccountId, amount: u64, reason: RewardReason) -> DispatchResult;

    /// 消费 Karma 用于功德（系统/其他 Pallet 调用）
    fn consume_karma_for_merit(who: &AccountId, amount: u64, action: MeritAction) -> DispatchResult;

    /// 获取用户总功德值
    fn total_merit_value(who: &AccountId) -> u64;

    /// 获取用户修为等级
    fn merit_level(who: &AccountId) -> u8;
}

/// 禅修（Meditation）相关接口：验证修行、发放奖励等
pub trait MeditationProvider<AccountId> {
    /// 完成禅修获得 Karma 奖励
    fn complete_meditation(who: &AccountId, duration_minutes: u32) -> DispatchResult;

    /// 验证修行任务完成
    fn verify_meditation_task(who: &AccountId, task_id: TaskId, proof: Vec<u8>) -> bool;
}
```

---

## Extrinsics（外部调用，用户发起）

```rust
/// 每日签到，按规则发放 Karma 奖励
/// - 防刷：结合时间提供者与最近一次签到记录
/// - 连续签到：可叠加倍数奖励
#[pallet::weight(T::WeightInfo::daily_checkin())]
pub fn daily_checkin(origin: OriginFor<T>) -> DispatchResult

/// 完成禅修任务，校验通过后发放 Karma 奖励
/// - 任务去重：防止重复领取
/// - 证明校验：调用 MeditationProvider 验证
#[pallet::weight(T::WeightInfo::complete_meditation_task())]
pub fn complete_meditation_task(
    origin: OriginFor<T>,
    task_id: TaskId,
    proof: Vec<u8>
) -> DispatchResult

/// 执行功德行为：从余额中消费 Karma，并累计为总功德值
/// - 自动更新等级：根据总功德值重算等级
/// - 记录历史：写入消费历史（时间、类型、数量、备注）
#[pallet::weight(T::WeightInfo::perform_merit_action())]
pub fn perform_merit_action(
    origin: OriginFor<T>,
    action: MeritAction,
    karma_amount: u64,
    description: Vec<u8>
) -> DispatchResult
```

---

## 内部函数（供系统/其他 Pallet 使用）

```rust
/// 自动奖励 Karma（无需用户签名）
/// - 用于任务系统、禅修系统、贡献激励等
pub fn auto_reward_karma(
    who: &T::AccountId,
    amount: u64,
    reason: RewardReason
) -> DispatchResult

/// 自动消费 Karma 用于功德（无需用户签名）
/// - 用于捐赠、祈福等由系统触发的功德行为
pub fn auto_consume_karma_for_merit(
    who: &T::AccountId,
    amount: u64,
    action: MeritAction
) -> DispatchResult

/// 依据总功德值计算修为等级
/// - 阈值来自 MeritLevelThresholds 配置
pub fn calculate_merit_level(total_merit: u64) -> u8
```

---

## 与其他 Pallet 的协作

- pallet_meditation（禅修）
  - 完成禅修 -> 调用 auto_reward_karma 发放奖励
  - 任务验证 -> 通过 MeditationProvider 防止作弊
  - 基于等级解锁更高阶修行内容

- pallet_rewards（奖励/成就）
  - 事件订阅 -> 根据 MeritValueUpdated/LevelUp 发放成就
  - 贡献激励 -> 调用 reward_karma

- pallet_governance（治理，可选）
  - 声誉权重 -> 根据修为等级调整发言/投票权重
  - 门槛限制 -> 设定最低等级的提案条件

---

## Genesis 配置

```rust
#[pallet::genesis_config]
pub struct GenesisConfig<T: Config> {
    /// 初始 Karma 分配
    pub initial_karma: Vec<(T::AccountId, u64)>,
    /// 初始总功德值
    pub initial_merit: Vec<(T::AccountId, u64)>,
}

#[pallet::genesis_build]
impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
    fn build(&self) {
        for (account, karma) in &self.initial_karma {
            KarmaBalances::<T>::insert(account, karma);
        }
        for (account, merit) in &self.initial_merit {
            TotalMeritValue::<T>::insert(account, merit);
            let level = Pallet::<T>::calculate_merit_level(*merit);
            UserMeritLevel::<T>::insert(account, level);
        }
    }
}
```

---

## 权重与基准测试

```rust
impl<T: Config> Pallet<T> {
    /// 每日签到权重估算
    pub fn weight_daily_checkin() -> Weight {
        // 读写：检查签到 + 写入奖励与记录
        T::DbWeight::get().reads_writes(2, 2)
    }

    /// 完成任务权重估算
    pub fn weight_complete_task() -> Weight {
        // 读写：校验任务 + 去重 + 发奖
        T::DbWeight::get().reads_writes(3, 3)
    }

    /// 功德行为权重估算
    pub fn weight_merit_action() -> Weight {
        // 读写：扣减余额 + 记账 + 计算等级 + 写入历史
        T::DbWeight::get().reads_writes(4, 4)
    }
}
```

---

## 迁移与升级

- 存储迁移：如字段结构调整、阈值策略变化等需提供迁移逻辑
- 数据完整性：升级期间确保功德记录与总功德值保持一致
- 版本管理：使用 StorageVersion 管理状态机升级

---

## API 使用示例

```rust
// 用户每日签到
// 返回：发放的 Karma 数量、连续签到天数
pallet_buddha_karma::daily_checkin(origin);

// 完成禅修任务（带证明）
// 返回：发放的 Karma 数量
pallet_buddha_karma::complete_meditation_task(origin, task_id, proof);

// 执行功德行为（消费 Karma）
// 返回：事件 MeritValueUpdated/LevelUp
pallet_buddha_karma::perform_merit_action(
    origin,
    MeritAction::Prayer,
    1_000,
    b"为众生祈福".to_vec()
);

// 查询 Karma 余额
let karma_balance = pallet_buddha_karma::karma_balances(account_id);

// 查询总功德值与等级
let total_merit = pallet_buddha_karma::total_merit_value(account_id);
let merit_level = pallet_buddha_karma::user_merit_level(account_id);

// 查询功德消费历史（自定义接口）
let history = pallet_buddha_karma::get_merit_consumption_history(account_id);

// 系统/其他 Pallet：奖励 Karma（无需签名）
pallet_buddha_karma::auto_reward_karma(&account_id, 500, RewardReason::Meditation);

// 系统/其他 Pallet：自动消费 Karma（无需签名）
pallet_buddha_karma::auto_consume_karma_for_merit(&account_id, 300, MeritAction::Donation);
```

---

## 测试建议

- 单元测试
  - 签到：限频、连续签到与奖励倍数
  - 任务：证明校验、去重、奖励正确性
  - 功德行为：余额扣减、历史记录、等级更新
  - 反滥用：零金额、溢出、频率限制

- 集成测试
  - 与 pallet_meditation/pallet_rewards 协同
  - 大规模账户/高频调用下的性能
  - 升级与迁移的向后兼容性

---

## 安全与权限

- 不可转移保障：任何转账路径一律失败
- 限频与防刷：通过时间提供者与策略限制高频调用
- 外部调用白名单：系统自动操作仅允许来自受信 Pallet
- 审计日志：关键事件均通过 Event 记录，便于链下索引与审计

---

## 与旧版文档的主要差异

1. 去除所有 BUD/价格/兑换相关内容，专注 Karma 作为非经济化声誉积分
2. 强化“功德消费 -> 总功德值累计 -> 等级提升”的闭环机制
3. 明确 Trait 接口，支持与禅修/奖励等 Pallet 的解耦协作
4. 加强防刷与不可转移的约束，避免投机与滥用
5. 提供更系统化的测试与升级建议