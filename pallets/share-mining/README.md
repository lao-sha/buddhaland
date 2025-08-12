# Share-Mining Pallet

## 功能概述

Share-Mining Pallet 实现链外内容分享挖矿激励机制：
- 用户提交包含佛境相关关键词的网页链接
- Offchain Worker 自动抓取验证链接内容
- 将奖金池（来自 Exchange 模块分配的 BUD）平均分给本轮获奖者

## 核心特性

- 奖金池账户（PotAccount）：由 runtime 常量提供，Exchange Pallet 会将一部分 BUD 转入此账户
- 去中心化验证：Offchain Worker 自动化内容抓取与关键词匹配
- 分批发放：支持大规模参与者的高效奖励分配

## 配置参数

- `PotAccount`: 奖金池接收账户
- `MaxUrlLen`: URL 最大长度限制
- `MaxParticipantsPerRound`: 每轮最大处理参与者数
- `HttpTimeoutMillis`: HTTP 请求超时时间
- `UnsignedPriority`: 无签名交易优先级

## 功能特性 (Features)

### 默认功能
- `std`: 标准库支持，包含所有依赖项的 std feature

### 可选功能
- `runtime-benchmarks`: 性能基准测试支持
  - 启用 frame-benchmarking 的 runtime-benchmarks feature
  - 提供性能测试和权重校准功能
  - 用于 Runtime 性能优化和权重计算
- `try-runtime`: 运行时状态验证和调试支持

## 存储结构

- `RoundId`: 当前轮次计数器
- `Pending`: 待验证队列（URL提交记录）
- `Winners`: 本轮合格参与者集合
- `WinnersCount`: 本轮获奖者计数

## 事件

- `LinkSubmitted(who, url)`: 用户提交链接
- `LinkVerified(who, url, matched)`: 链下验证完成
- `RewardsDistributed(round_id, winners, per_user, total_paid)`: 完成奖励分配
- `PotIncreased(amount)`: 奖金池资金增加

## 外部调用接口

1. `submit_link(url: Vec<u8>)`：用户提交待验证链接
2. `submit_verification(who, url, submitted_at, matched)`：Offchain Worker 回填验证结果（无签名交易）
3. `distribute(max_winners: u32)`：从奖金池读取余额，平均分配给本轮 Winners 的前 N 位（分批处理）

## 工作流程

### 用户参与流程
1. 用户调用 `submit_link()` 提交符合要求的网页链接
2. Offchain Worker 在后台抓取链接内容，检查是否包含关键词：["佛境", "冥想", "修心", "禅修"]
3. 匹配成功的用户自动加入本轮 Winners 队列

### 奖励分配流程
1. 任何人可调用 `distribute()` 触发奖金池分配
2. 算法：per_user = pot_balance / winners_in_batch
3. remainder 保留在奖金池
4. 完成后清空已分配的 Winners，若全部处理完毕则自增轮次

## 安全考虑

- URL 长度限制防止存储攻击
- Offchain Worker 使用无签名交易避免 gas 消耗
- 分批处理避免单个区块耗时过长
- PotAccount 应由治理或运维确保仅接受来自 Exchange 的分配

## 与 Exchange 集成
- Exchange Pallet 已支持第四个分配目标：Share-Mining 奖金池
- runtime 需在 Exchange 的 `Config` 中提供 `ShareMiningBps` 配置 Share-Mining 分配比例
- Exchange 会直接转账 BUD 到此 Pallet 的 `PotAccount`

## 公开模块函数

- `pot_account()`: 获取奖金池账户ID - 供其他 pallet 获取转账目标
- `pot_balance()`: 获取奖金池当前余额

## 运行时配置示例

```rust
impl pallet_share_mining::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type PotAccount = ShareMiningPotAccountId;
    type MaxUrlLen = ConstU32<512>;
    type MaxParticipantsPerRound = ConstU32<50>;
    type HttpTimeoutMillis = ConstU64<10000>;
    type UnsignedPriority = ConstU32<100>;
}
```

### Feature 配置

在 `Cargo.toml` 中启用相应功能：

```toml
[dependencies]
pallet-share-mining = { path = "../pallets/share-mining", default-features = false }

[features]
runtime-benchmarks = [
    "pallet-share-mining/runtime-benchmarks",
    # ... 其他 pallet 的 runtime-benchmarks
]
```

## 总结

Share-Mining Pallet 通过链外内容验证和链上奖励分配的结合，为佛境生态提供了去中心化的用户参与激励机制，推动了佛境相关内容的传播与社区建设。新增的 runtime-benchmarks 功能支持为性能优化和权重校准提供了技术基础。