# Reward Pallet (奖励系统)

佛境项目的智能奖励系统，专门负责基于 meditation pallet 产生的有效禅修冥想数据，计算并发放 Karma 福缘值奖励。

## 设计理念

基于 Polkadot SDK 最佳实践的**职责分离原则**，将禅修数据验证与奖励计算解耦：
- **Meditation Pallet**：专注于冥想会话数据的验证与存储
- **Reward Pallet**：专注于基于冥想质量的动态奖励算法

## 功能特性

### 1. 多维度奖励算法
- **时长奖励**：基础奖励按冥想分钟数线性计算
- **深度奖励**：根据平均冥想深度（Alpha/Theta 波强度）给予加成
- **专注奖励**：基于专注程度稳定性的额外奖励
- **质量奖励**：脑波数据质量分数的奖励系数

### 2. 可配置的奖励权重
```rust
pub struct RewardWeights {
    pub base_per_minute: u128,  // 每分钟基础奖励
    pub depth_weight: u8,       // 深度加成权重
    pub focus_weight: u8,       // 专注加成权重
    pub quality_weight: u8,     // 质量分加成权重
}
```

### 3. 安全性保障
- **验证机制**：可配置仅对已验证的冥想会话发放奖励
- **防重复奖励**：与 meditation pallet 协同确保每个会话仅奖励一次
- **溢出保护**：数值计算包含溢出检查

## 奖励计算公式

```
总奖励 = 基础奖励 + 深度加成 + 专注加成 + 质量加成

其中：
- 基础奖励 = 冥想分钟数 × base_per_minute
- 深度加成 = 平均冥想深度(0-100) × depth_weight  
- 专注加成 = 平均专注程度(0-100) × focus_weight
- 质量加成 = 脑波质量分数(0-100) × quality_weight
```

## 集成接口

### 与 Meditation Pallet 集成
```rust
// 监听 meditation pallet 的事件，自动触发奖励计算
// 或通过手动调用 grant_for_session 实现
```

### 与 Karma Pallet 集成
```rust
// 通过 KarmaProvider trait 发放奖励
T::Karma::reward_karma(&who, amount, RewardReason::Meditation)
```

## 配置示例

```rust
impl pallet_reward::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type RewardWeights = ConstU128<RewardWeights {
        base_per_minute: 10,   // 每分钟10个Karma基础奖励
        depth_weight: 2,       // 深度每分值2个Karma加成
        focus_weight: 1,       // 专注每分值1个Karma加成  
        quality_weight: 1,     // 质量每分值1个Karma加成
    }>;
    type OnlyRewardVerified = ConstBool<true>;  // 仅奖励已验证会话
    type Karma = Karma;
    type Moment = Moment;
}
```

## 使用场景

### 1. 自动奖励（推荐）
通过事件监听或 hooks 机制，在 meditation pallet 成功提交会话后自动触发奖励计算。

### 2. 手动奖励
管理员或治理模块可通过 `grant_for_session` 手动触发奖励发放，适用于：
- 延迟奖励验证
- 批量处理历史数据
- 特殊奖励调整

### 3. 奖励重算
支持对已存储的冥想会话重新计算奖励（例如算法参数调整后）。

## 事件监听

```rust
pub enum Event<T: Config> {
    /// 已根据冥想会话发放奖励 (who, session_id, amount)
    RewardGranted(T::AccountId, SessionId, u128),
}
```

## 安全考虑

- **权限控制**：`grant_for_session` 需要 root 权限，防止滥用
- **数据验证**：与 meditation pallet 数据格式严格对应
- **防重复**：建议在上层逻辑中避免对同一会话重复奖励
- **参数合理性**：RewardWeights 参数应经过充分测试，避免通胀

## 扩展性

该 pallet 设计为模块化，可轻松扩展：
- 添加新的奖励因子（如连续性、进步幅度）
- 实现复杂的奖励曲线（指数、对数等）
- 支持不同冥想类型的差异化奖励
- 集成更多外部数据源（天气、时间等）

---

## 与现有 Karma 系统的关系

- **补充关系**：不替代 karma pallet 的现有功能，仅专注于冥想奖励
- **接口兼容**：通过 KarmaProvider trait 与 karma pallet 通信
- **数据一致**：所有奖励最终记录在 karma pallet 的存储中
- **事件同步**：reward 事件与 karma 事件可联合追踪完整奖励链路