# Share-Mining Pallet

面向“分享挖矿”的链上激励模块：
- 用户提交分享链接
- 链下 Offchain Worker 抓取链接内容，判断是否包含关键词：“佛境”“冥想”“修心”“禅修”
- 若匹配，登记为当前轮次的获奖者
- 将奖金池（来自 Exchange 模块分配的 BUD）平均分给本轮获奖者

## 术语与外部依赖
- 奖金池账户（PotAccount）：由 runtime 常量提供，Exchange Pallet 会将一部分 BUD 划入此账户
- 代币接口：使用 Balances 的 Currency 接口进行转账支付
- Offchain Worker：使用 Substrate Offchain HTTP 进行抓取与匹配

## 配置项
- `Currency`: BUD代币接口
- `PotAccount`: 奖金池接收账户
- `MaxUrlLen`: 最大 URL 字节长度
- `MaxParticipantsPerRound`: 每轮最多参与者（用于分批发放，控制区块耗时）
- `HttpTimeoutMillis`: HTTP 请求超时时间（毫秒）
- `UnsignedPriority`: 无签名交易优先级

## 存储
- `RoundId`: 当前轮次 ID
- `Pending`: 待验证提交（who, url, at）
- `Winners`: 本轮合格参与者 set
- `WinnersCount`: 本轮合格参与者数量

## 事件
- `LinkSubmitted(who, url)`
- `LinkVerified(who, url, matched)`
- `RewardsDistributed(round_id, winners, per_user, total_paid)`

## Extrinsics
1. `submit_link(url: Vec<u8>)`：用户提交链接，进入 Pending 队列（长度受 `MaxUrlLen` 限制）
2. `submit_verification(who, url, submitted_at, matched)`：OCW 无签名回填验证结果（内部使用）
3. `distribute(max_winners: u32)`：从奖金池读取余额，平均分配给本轮 Winners 的前 N 位（分批处理）

## Offchain Worker
- 在每个区块执行，扫描少量 Pending 提交（默认5个），对 URL 发起 HTTP GET
- 解析响应体为 UTF-8 文本，匹配关键词
- 提交 `submit_verification` 无签名交易写回链上

## 分配规则
- per_user = pot_balance / winners_in_batch
- remainder 保留在奖金池
- 每次调用 `distribute` 支持处理部分获奖者，完成本轮后自增 `RoundId`

## 安全性
- 无签名交易通过 `ValidateUnsigned` 校验 Pending 是否存在，防止重放
- URL 长度受限，避免 DoS 风险
- 分批发放避免单块内处理过多账户
- PotAccount 应由治理或运维确保仅接受来自 Exchange 的分配

## 与 Exchange 集成
- Exchange Pallet 已支持第四个分配目标：Share-Mining 奖金池
- runtime 需在 Exchange 的 `Config` 中提供 `ShareMiningAccount` 与 `ShareMiningBps`

## 测试建议
- 提交/验证：正确/错误 URL、重复提交、超长 URL
- Offchain：HTTP 超时、网络错误、编码异常
- 分配：多用户、余额边界、批量处理、无获奖者