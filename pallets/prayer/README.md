# Prayer Pallet

本 Pallet 提供“祈福（Prayer）”行为的上链接口：用户提交祈福内容摘要并消耗一定的 Karma（功德），链上发出事件以供前端展示或索引。祈福内容不在链上存明文，建议以摘要（哈希或紧凑摘要）形式提交。

- 职责：为“祈福行为”提供消费 Karma 的入口，并发出事件
- 依赖：通过 Karma Pallet 的 KarmaProvider 接口完成功德消费
- 数据最小化：不存储祈福明文，仅事件中记录摘要与消费数额

---

## 1. 目标与职责边界

- 负责：
  - 接收用户祈福请求（pray extrinsic）
  - 按传入金额或默认金额消耗 Karma
  - 记录祈福事件（Prayed）
- 不负责：
  - 祈福内容明文存储（仅存摘要，避免隐私与存储膨胀）
  - Karma 余额与结算（由 Karma Pallet 负责）
  - 高级策略（配额、频控、黑名单等；可由上层策略或其他 pallet 扩展）

---

## 2. 术语与对齐

- Karma ↔ 功德（余额与发放/消费由 Karma Pallet 管理）
- MeritAction::Prayer ↔ 祈福行为
- consume_karma_for_merit ↔ 功德消费（用于具体功德行为）
- Prayer Content 摘要 ↔ 祈福内容的哈希或简要摘要（Vec<u8>）

推荐在链下对祈福内容进行哈希（如 Blake2 或 SHA-256），上链仅携带其摘要，保护隐私并控制存储体积。

---

## 3. 类型与配置（Config）

- type RuntimeEvent
- #[pallet::constant] type DefaultPrayerCost: Get<KarmaBalance>
  - 祈福的默认消耗功德值（当 extrinsic 未指定 amount 时使用）
- 依赖类型（来自 Karma Pallet）
  - KarmaBalance：功德余额类型
  - KarmaProvider：提供 reward_karma、consume_karma_for_merit 等接口
  - MeritAction：功德行为枚举，使用 MeritAction::Prayer

---

## 4. 存储

- 无持久化存储（stateless）
  - 仅通过事件对外暴露祈福行为结果

---

## 5. 事件（Event）

- Prayed(AccountId, KarmaBalance, Vec<u8>)
  - 当一次祈福成功消费功德后触发
  - 字段含义：
    - AccountId：祈福发起账户
    - KarmaBalance：实际消耗的功德值
    - Vec<u8>：祈福内容摘要（建议为哈希或紧凑摘要）

---

## 6. 错误（Error）

- EmptyPrayer：祈福内容摘要为空（content.is_empty()）
- 运行时会将 Karma 消费失败映射为 DispatchError::Other("ConsumeFailed")

---

## 7. 可调用函数（Extrinsics）

1) pray(origin, content: Vec<u8>, amount: Option<KarmaBalance>) -> DispatchResultWithPostInfo
   - 权限：签名交易（ensure_signed）
   - 行为：
     - 校验 content 非空
     - 计算 cost = amount.unwrap_or(DefaultPrayerCost)
     - 调用 KarmaProvider::consume_karma_for_merit(who, cost, MeritAction::Prayer)
     - 触发事件 Event::Prayed(who, cost, content)
   - 失败：
     - Error::EmptyPrayer
     - DispatchError::Other("ConsumeFailed")（当 Karma 侧消费失败时）
   - 权重：当前实现为常量 10_000（占位，建议基准测试后在 runtime 中提供 WeightInfo 实现）

---

## 8. 权重（Weights）

- 当前实现使用固定值 #[pallet::weight(10_000)]
- 建议：
  - 使用 frame-benchmarking 为 pray 流程做基准测试
  - 在 runtime 中实现 WeightInfo 并替换固定值

---

## 9. 运行时集成示例

以下示例展示如何在 runtime 中配置 Prayer Pallet 的默认功德消耗值，并确保 Karma Pallet 被正确包含以提供 KarmaProvider 能力。

```rust:runtime%2Fsrc%2Flib.rs
// 函数级注释：为 Prayer Pallet 提供配置，指定默认祈福消耗值。
// 注意：需确保 runtime 已集成 Karma Pallet，并实现其 Config 与 KarmaProvider。

parameter_types! {
    // 函数级注释：默认每次祈福消耗的功德值，可按经济模型调整。
    pub const DefaultPrayerCost: pallet_karma::KarmaBalance = 10;
}

impl pallet_prayer::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    // 函数级注释：默认祈福消耗功德，来自上面的 parameter_types。
    type DefaultPrayerCost = DefaultPrayerCost;
}

// 函数级注释：Karma Pallet 的集成（示意），需要在 runtime 中包含，
// 以便 Prayer Pallet 能调用 KarmaProvider::consume_karma_for_merit。
impl pallet_karma::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    // ... 其他 Karma 相关配置 ...
}
```

---

## 10. 前端/调用示例（polkadot.js）

以下示例展示如何在前端以摘要方式提交祈福，并可选择覆盖默认消耗。

```typescript:docs%2Fprayer_example.ts
/**
 * 函数级注释：
 * 通过 polkadot.js 提交祈福交易。
 * - contentDigest: 祈福内容的链下哈希或紧凑摘要（建议使用 Blake2 或 SHA-256）
 * - amount: 可选覆盖默认消耗的功德值；不传则使用链上默认值
 */
async function pray(api: any, signer: any, contentDigest: Uint8Array, amount?: bigint) {
  const tx = amount !== undefined
    ? api.tx.prayer.pray(contentDigest, amount)      // 显式金额
    : api.tx.prayer.pray(contentDigest, null);       // 使用默认金额（Option::None）

  await tx.signAndSend(signer, ({ status, events }: any) => {
    if (status.isInBlock || status.isFinalized) {
      console.log('Prayer submitted:', status.toString());
      for (const { event } of events) {
        if (api.events.prayer.Prayed.is(event)) {
          const [who, cost, content] = event.data;
          console.log(`Prayed by ${who}, cost: ${cost}, contentDigest: ${Buffer.from(content).toString('hex')}`);
        }
      }
    }
  });
}
```

---

## 11. 安全与隐私

- 内容最小化：不在链上存储祈福明文，建议仅上传摘要（哈希或简短摘要）
- 大小限制：请在链下控制摘要大小（如 32 字节哈希），避免滥用存储
- 失败处理：Karma 消费失败会返回 DispatchError::Other("ConsumeFailed")，前端需提示用户余额或权限问题

---

## 12. 测试建议

- 单元测试
  - content 为空时应返回 Error::EmptyPrayer
  - 指定 amount 与使用默认值两种场景均能正确消费功德
  - KarmaProvider::consume_karma_for_merit 失败时，extrinsic 返回错误
  - 成功路径应触发 Event::Prayed，事件数据与传入一致
- 模拟测试（integration）
  - 在 runtime 中设置不同的 DefaultPrayerCost
  - 覆盖不同账户 Karma 余额与权限情况

---

## 13. 序列图（简化）