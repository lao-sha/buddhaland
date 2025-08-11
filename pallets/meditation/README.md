# Meditation Pallet

本 Pallet 用于在链上记录冥想（禅修）会话摘要，并提供基本阈值校验、会话验证及与奖励模块（如 Karma）的可插拔集成接口。

- 目标：
  - 上链冥想会话摘要（仅存摘要，不存原始脑波数据）
  - 基础反作弊阈值校验（最小时长、最低质量分）
  - 与奖励模块集成（通过 RewardHook 回调，支持对有效会话进行奖励）
  - 提供简要的历史数据索引（通过双映射按用户+会话ID读取）

- 依赖：
  - FRAME System Pallet
  - Timestamp Pallet（用于填充会话开始时间）
  - Karma Pallet（通过 KarmaProvider trait 集成——可选）
  - 自定义 RewardHook（由外部 Pallet 实现，用于奖励逻辑）

- 关键点：
  - 使用 StorageDoubleMap 存储用户多会话数据
  - 通过 Root 权限进行会话“验证”标记（预留与链下证明/ZK集成空间）
  - 奖励逻辑通过 RewardHook 解耦，Meditation Pallet 仅负责触发回调与数据记录

---

## 1. 类型定义

- SessionId = u64
- MeditationMetrics（冥想指标，0-100 简化分制）：
  - meditation_depth: u8
  - focus_level: u8
  - alpha_power: u8
  - theta_power: u8

- MeditationSession<Moment>（会话摘要）：
  - start_time: Moment
  - duration_minutes: u32
  - avg_meditation_depth: u8
  - avg_focus_level: u8
  - peak_meditation_depth: u8
  - brainwave_quality_score: u8
  - verified: bool

- MeditationRewardReason（奖励原因占位，面向 Karma 集成预留）：
  - Meditation

- RewardHook<AccountId, Moment>（奖励回调 Trait）：
  - fn on_session_submitted(who, session_id, metrics, session)
  - 说明：外部奖励模块实现该回调，根据上报会话决定是否发放奖励（例如 Karma）

---

## 2. 存储项

- NextSessionId<T: Config>
  - Map: AccountId -> SessionId
  - 描述：为每个用户维护自增的下一个会话 ID

- Sessions<T: Config>
  - DoubleMap: (AccountId, SessionId) -> MeditationSession<Moment>
  - 描述：存储用户的每条会话摘要数据

---

## 3. 事件（Event）

- SessionSubmitted(AccountId, SessionId)
  - 成功提交会话后触发

- SessionVerified(AccountId, SessionId)
  - 成功将会话标记为“已验证”后触发（Root 权限）

- KarmaRewarded(AccountId, SessionId, u128)
  - 预留事件，用于直接在本 Pallet 内发放 Karma 时记录（当前版本奖励通过 RewardHook 回调实现，不在本 Pallet 内直接发放）

---

## 4. 错误（Error）

- TooShort：会话时长不足最小阈值
- LowQuality：脑波质量分低于阈值
- ZeroReward：预留错误类型（当前版本未在本 Pallet 内使用）
- Overflow：预留错误类型（当前版本未在本 Pallet 内使用）
- AlreadyVerified：会话已被标记为 verified，无法重复标记

---

## 5. 配置项（Config）

- type RuntimeEvent
- type WeightInfo: WeightInfo
- type Moment: Parameter + Default + MaxEncodedLen + TypeInfo + Copy
- #[pallet::constant] type EnableKarmaReward: Get<bool>
  - 预留开关位：是否启用 Karma 奖励（当前奖励逻辑由 RewardHook 实施）
- #[pallet::constant] type BaseRewardPerMinute: Get<u128>
  - 预留奖励基数（每分钟奖励量）
- #[pallet::constant] type MinDurationMinutes: Get<u32>
  - 会话最小时长阈值
- #[pallet::constant] type MinQualityScore: Get<u8>
  - 脑波质量最小阈值
- type Karma: KarmaProvider<Self::AccountId>
  - 与 Karma Pallet 的抽象集成接口
- type Reward: RewardHook<Self::AccountId, Self::Moment>
  - 奖励回调实现方（由外部 Pallet 提供）

---

## 6. 可调用函数（Extrinsics）

1) submit_session(origin, metrics: MeditationMetrics, session: MeditationSession<Moment>) -> ()
   - 权限：签名账号（ensure_signed）
   - 功能：
     - 校验会话时长和质量阈值（MinDurationMinutes、MinQualityScore）
     - 若 start_time 未设置，自动使用 Timestamp Pallet 的 now() 作为开始时间
     - 写入存储（NextSessionId 分配 ID，Sessions 保存记录）
     - 触发 Event::SessionSubmitted
     - 调用 T::Reward::on_session_submitted(...) 回调，交给外部奖励模块处理
   - 失败：
     - TooShort / LowQuality
   - 权重：T::WeightInfo::submit_session()

2) verify_session(origin, who: AccountId, session_id: SessionId) -> ()
   - 权限：Root（ensure_root）
   - 功能：
     - 将指定用户的会话标记为 verified = true
     - 触发 Event::SessionVerified
   - 失败：
     - AlreadyVerified
   - 权重：T::WeightInfo::verify_session()

---

## 7. 与其他 Pallet 的集成

- Timestamp Pallet
  - 用于在提交时补齐会话开始时间：当 session.start_time 为默认值时，使用 now() 赋值

- Karma Pallet（可选）
  - 通过 KarmaProvider trait 集成，相关常量（EnableKarmaReward、BaseRewardPerMinute）用于奖励计算参数的约定
  - 当前版本的奖励实际由 RewardHook 回调实现；如果未来在本 Pallet 内直接发放奖励，可使用 KarmaRewarded 事件进行记录

- Reward Pallet（实现 RewardHook）
  - 实现 RewardHook::on_session_submitted(...)，根据 metrics 与 session 决定是否发放奖励（例如按 duration_minutes 与质量分计算）
  - 好处：与业务奖励解耦，Meditation Pallet 保持职责单一

---

## 8. 权重（Weight）与基准

- WeightInfo Trait
  - submit_session() -> Weight
  - verify_session() -> Weight
- 默认实现（for ()）提供占位权重：
  - submit_session: 10_000
  - verify_session: 5_000
- 建议：
  - 使用 frame-benchmarking 对不同输入规模进行基准测试
  - 在 runtime 中为 WeightInfo 提供基于基准结果的实现

---

## 9. 典型数据结构（前后端约定示例）

- MeditationMetrics（JSON 示例）：
  ```json
  {
    "meditation_depth": 72,
    "focus_level": 65,
    "alpha_power": 58,
    "theta_power": 61
  }