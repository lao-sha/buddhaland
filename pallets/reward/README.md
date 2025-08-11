# Reward Pallet

本 Pallet 的职责是对接 Meditation Pallet 的会话提交流程，依据会话摘要与指标计算 Karma 奖励并发放：
- 在被 Meditation Pallet 回调（RewardHook）时自动计算与发放奖励；
- 也支持 Root 手动触发对任意会话的奖励发放（grant_for_session）。

该 Pallet 不存储会话数据，仅消费由 Meditation Pallet 上报的会话摘要与指标，并调用 Karma Pallet 发放奖励。
+ 
+ 与《佛境文档.md》双 Pallet 架构对齐说明：
+ - 双 Pallet 架构：Meditation（数据/验证）+ Reward（计算/发放），余额/功德由 Karma 管理
+ - “仅奖励验证通过”策略：对应本 Pallet 的常量 OnlyRewardVerified（true 时仅对 verified = true 的会话发放）

---

## 1. 范围与目标

- 职责范围
  - 奖励计算与发放：对冥想会话按配置好的权重进行打分计算奖励量，调用 Karma Pallet 发放。
  - 触发来源：
    - 自动：在 Meditation Pallet 成功提交会话后，通过 RewardHook 回调触发。
    - 手动：通过 extrinsic grant_for_session 由 Root 发起。

- 非职责范围
  - 不负责会话数据的存储（由 Meditation Pallet 提供）
  - 不负责会话合法性的判定（仅通过 OnlyRewardVerified 开关做“verified”必要性检查）
  - 不负责余额/货币管理（通过 KarmaProvider 调用发放）

---

## 2. 类型定义

- RewardWeights
  - base_per_minute: u128
  - depth_weight: u8
  - focus_weight: u8
  - quality_weight: u8
  - 用途：在 reward 侧对时长与指标的加权配置，控制奖励量计算的权重与基数

- 复用的外部类型
  - MeditationMetrics（来自 meditation pallet）
  - MeditationSession<Moment>（来自 meditation pallet）
  - SessionId = u64（来自 meditation pallet）
  - RewardReason::Meditation（来自 karma pallet）
+ 
+ ### 2.1 与《佛境文档.md》术语/指标对齐
+ - 冥想深度（Meditation Depth）↔ metrics.meditation_depth（u8，0-100）
+ - 专注程度（Focus Level）↔ metrics.focus_level（u8，0-100）
+ - 脑波质量评分（Brainwave Quality Score）↔ session.brainwave_quality_score（u8，0-100）
+ - 会话时长（Duration, 分钟）↔ session.duration_minutes（u32，单位：分钟）
+ - 平均冥想深度 ↔ session.avg_meditation_depth（u8，0-100）
+ - 平均专注程度 ↔ session.avg_focus_level（u8，0-100）
+ - 峰值冥想深度 ↔ session.peak_meditation_depth（u8，0-100）
+ - 验证状态（是否通过反作弊与真实性校验）↔ session.verified（bool）

---

## 3. 配置项（Config）

- type RuntimeEvent
- type WeightInfo: WeightInfo
- #[pallet::constant] type RewardWeights: Get<RewardWeights>
  - 奖励计算权重配置项
- #[pallet::constant] type OnlyRewardVerified: Get<bool>
  - 若为 true，则仅对 verified == true 的会话发放奖励
- type Karma: KarmaProvider<Self::AccountId>
  - 发放 Karma 的抽象接口
- type Moment: Parameter + Default + MaxEncodedLen + TypeInfo + Copy
  - 与 Meditation Pallet 的 Moment 类型兼容，便于事件/计算中使用

---

## 4. 存储项

- 当前版本无链上存储（stateless），仅在触发时计算并调用 Karma 发放。

---

## 5. 事件（Event）

- RewardGranted(AccountId, SessionId, u128)
  - 当某会话被授予奖励时触发，记录 (who, session_id, amount)

---

## 6. 错误（Error）

- ZeroReward：计算结果为 0，不予发放
- Overflow：发放过程中上游返回溢出/错误
- NotVerified：在 OnlyRewardVerified = true 且会话未验证时拒绝发放

---

## 7. 可调用函数（Extrinsics）

1) grant_for_session(origin, who: AccountId, session_id: SessionId, metrics: MeditationMetrics, session: MeditationSession<Moment>) -> ()
   - 权限：Root（ensure_root）
   - 功能：
     - 根据传入的 metrics + session 计算奖励，若 amount > 0 则调用 KarmaProvider 发放
     - 当 OnlyRewardVerified = true 时，会检查 session.verified 必须为 true
     - 触发 Event::RewardGranted
   - 失败：
     - ZeroReward / NotVerified / Overflow
   - 权重：T::WeightInfo::grant_for_session()

函数级注释（实现思路说明）：
- grant_for_session：
  - 输入：用户账户、会话 ID、简化指标与会话摘要
  - 校验：OnlyRewardVerified 时需 verified = true
  - 计算：调用内部 calculate_reward_internal（线性组合方式）
  - 发放：通过 T::Karma::reward_karma(who, amount, RewardReason::Meditation)
  - 记录：成功后发出 RewardGranted 事件

---

## 8. Hook 集成（与 Meditation Pallet）

本 Pallet 实现了 Meditation Pallet 的 RewardHook：在冥想会话提交成功后，Meditation Pallet 会调用 on_session_submitted 进行奖励发放。

- on_session_submitted(who, session_id, metrics, session)
  - 若 OnlyRewardVerified = true 且 session.verified = false，则直接返回不发放
  - 计算奖励 amount = calculate_reward_internal(...)
  - amount > 0 时尝试发放 Karma，并触发 RewardGranted 事件
  - 如发放失败，静默忽略错误以不影响 Meditation Pallet 主流程

函数级注释（实现思路说明）：
- on_session_submitted：
  - 场景：由 Meditation Pallet 自动回调，无需链上外部调用
  - 策略：最小侵入，出错静默，避免拖累会话提交流程
  - 保障：通过 OnlyRewardVerified 控制是否强制“verified 后奖励”
+ 
+ 名称映射（与《佛境文档.md》一致性）：
+ - 文档中的“Reward::grant_karma(who, amount, RewardReason::Meditation)”在链上由 Karma Pallet 暴露的 KarmaProvider::reward_karma 实现，本 Reward Pallet 在回调或手动发放时调用该接口完成实际发放。
+ - 文档中的 “Meditation::submit_session(...)” 对应 meditation pallet 的 submit_session extrinsic（当前实现为 submit_session(origin, metrics, session)），Reward Pallet 通过 RewardHook 被动接收回调。

---

## 9. 奖励计算公式

内部计算函数 calculate_reward_internal(metrics, session) 使用线性组合示例：
- base = session.duration_minutes * RewardWeights.base_per_minute
- depth = metrics.meditation_depth * RewardWeights.depth_weight
- focus = metrics.focus_level * RewardWeights.focus_weight
- quality = session.brainwave_quality_score * RewardWeights.quality_weight
- amount = base + depth + focus + quality（均采用 saturating_* 防溢出）

函数级注释（实现思路说明）：
- calculate_reward_internal：
  - 目标：给出可解释的线性计算，稳定且可通过常量配置调整
  - 可扩展：未来可替换为更复杂的多维算法、非线性函数或分段函数
+ 
+ 术语对齐（以《佛境文档.md》为准的中文名）：
+ - 冥想深度（meditation_depth）
+ - 专注程度（focus_level）
+ - 脑波质量评分（brainwave_quality_score）
+ - 会话时长（duration_minutes，单位：分钟）

---

## 10. 权重（Weights）

- WeightInfo Trait
  - grant_for_session() -> Weight
- 默认实现（for ()）提供占位权重：
  - grant_for_session: 10_000
- 建议：
  - 使用 frame-benchmarking 结合真实参数进行基准测试，并在 runtime 中提供实际权重实现

---

## 11. 运行时集成

- 在 runtime 中为 Meditation Pallet 指定 RewardHook 的实现为 Reward Pallet：
  ```rust
  // 函数级注释：此片段展示 runtime 中将 Meditation 的 RewardHook 指向 Reward Pallet
  // 使得 meditation.submit_session 成功后会回调 reward::Pallet 进行奖励。

  impl pallet_meditation::Config for Runtime {
      type RuntimeEvent = RuntimeEvent;
      type WeightInfo = pallet_meditation::weights::SubstrateWeight<Runtime>;
      type Moment = Moment;
      type EnableKarmaReward = EnableKarmaReward;
      type BaseRewardPerMinute = BaseRewardPerMinute;
      type MinDurationMinutes = MinDurationMinutes;
      type MinQualityScore = MinQualityScore;
      type Karma = Karma; // 由 runtime 提供的 KarmaProvider 实现
      type Reward = pallet_reward::Pallet<Runtime>; // 关键：指向本 Pallet
  }

  impl pallet_reward::Config for Runtime {
      type RuntimeEvent = RuntimeEvent;
      type WeightInfo = pallet_reward::weights::SubstrateWeight<Runtime>;
      type RewardWeights = RewardWeightsConst;      // const 实现 Get<RewardWeights>
      type OnlyRewardVerified = OnlyRewardVerified; // const 实现 Get<bool>
      type Karma = Karma;                           // 由 runtime 提供的 KarmaProvider 实现
      type Moment = Moment;                         // 与 meditation 的 Moment 保持一致
  }
  ```