### Karma Pallet 概览

Karma 是佛境项目的修为/声誉积分系统，管理用户的 Karma（业力/福缘值）作为不可转移的个人修行与功德累积指标。Karma 不支持任何形式的转账或交易，也不与任何代币挂钩；它通过链上行为（如签到、修行、任务、功德行为等）在各业务 Pallet 内部发放与消费，并以此累计“总功德值”与“修为等级”。

本实现已接入“可复用的全局授权/投票中心” pallet-authorizer：仅被授权的“调用账户”（通常为各业务 Pallet 的模块账户）才可调用内部增发/消费接口。授权通过 BUD 持币投票完成，避免 Root/Sudo 集中管理。

---

### 功能概述（与实现现状对齐）
- 不可转移的声誉积分：Karma 绑定账户，仅能由被授权业务逻辑增发/消费。
- 永久记录与等级：消费累加“总功德值”，并按阈值曲线自动计算“修为等级”。
- 跨 Pallet 友好：暴露内部 Trait（`gain`/`spend`），由其他 Pallet 在链上逻辑中直接调用（非 extrinsic）。
- 授权治理（已替换 Root/Sudo）：通过 `pallet-authorizer` 按命名空间治理白名单，BUD 持币投票决定增/删授权。
- 黑洞统计：所有被消费的 Karma 额外累加到全网黑洞计数，可用于全局数据看板与治理分析。
- 初始发行：0。

---

### 存储与类型
- `KarmaOf<AccountId -> Balance>`：账户当前可用 Karma 余额。
- `TotalMeritOf<AccountId -> Balance>`：账户累计功德值（随消费累加，长期保存）。
- `LevelOf<AccountId -> u32>`：账户修为等级（由累计功德值推导）。
- `HistoryOf<AccountId -> BoundedVec<KarmaRecord, HistoryMaxLen>]`：账户历史（获取/消费、数量、余额、总功德、等级、时间、备注）。
- `TotalBurned<Balance>`：全网被消费的 Karma 总量（黑洞统计）。

记录结构（关键字段）：
- `event_type`（Gain/Spend）、`amount`、`total_karma_after`、`total_merit_after`、`level_after`、`timestamp`（区块高度）、`memo`（`BoundedVec<u8, MaxMemoLen>`）。

配置参数（由 runtime 提供）：
- `KarmaBalance`（通常为 u128）、`HistoryMaxLen`、`MaxMemoLen`；
- `AuthorizerNamespace: Get<[u8;8]>` 指定授权中心的命名空间（建议与 Karma 的 PalletId 字节一致，如 `b"karma___"`）。

---

### 对外接口（供 Pallet 调用，非 extrinsic）
- `KarmaCurrency::gain(origin_caller, who, amount, memo)`：为 `who` 增加 Karma。
- `KarmaCurrency::spend(origin_caller, who, amount, memo)`：为 `who` 消费 Karma（余额扣减、累计功德+等级更新、黑洞总量+1）。

访问控制：
- 内部会校验 `origin_caller` 是否在授权中心 `pallet-authorizer` 的白名单中（命名空间 + 账户）。
- 只有白名单账户（通常是业务 Pallet 的“模块账户”）可调用上述接口。

模块账户建议：
```rust
use frame_support::PalletId;
use sp_runtime::traits::AccountIdConversion;

const MY_PALLET_ID: PalletId = PalletId(*b"budd/sin");
let module_account: T::AccountId = MY_PALLET_ID.into_account_truncating();
// 由 BUD 持币投票通过的提案将该账户加入 karma 命名空间白名单后，即可调用：
pallet_karma::Pallet::<T>::gain(&module_account, &user, amount, b"signin".to_vec())?;
```

---

### 授权治理（pallet-authorizer）对接
- 命名空间（Namespace）：8 字节固定标识，Karma 在 runtime 中通过：
```rust
parameter_types! {
    pub const KarmaNsBytes: [u8; 8] = *b"karma___";
}
impl pallet_karma::Config for Runtime {
    type AuthorizerNamespace = KarmaNsBytes;
    // 其余略
}
```
- 白名单变更：提交“添加/移除授权账户”的提案，由 BUD 持币投票，截止后执行。

---

### 等级规则（示例实现）
- 阈值阶梯：起始阈值 100，每跨越一个阈值等级 +1，下一阈值按“翻倍 + 100”增长，上限 100 级。
- 计算轻量、确定性强，适合链上逻辑；可在未来改为参数化或查表方式以获得更灵活曲线。

---

### 事件与错误
- 事件：
  - `KarmaGained { account, amount, new_balance }`
  - `KarmaSpent { account, amount, new_balance, new_total_merit, new_level }`
- 错误：
  - `NotAuthorized`、`InsufficientKarma`、`HistoryOverflow`、`MemoTooLong`

---

### 与旧版差异（重要）
- 移除 Root/Sudo 维护白名单的 extrinsic；统一改为 `pallet-authorizer` 通过 BUD 持币投票治理授权。
- 新增黑洞总量 `TotalBurned`，用于统计全网被消费的 Karma 总量。
- 历史备注 `memo` 改为有界 `BoundedVec<u8, MaxMemoLen>`，避免存储膨胀和编码长度不确定问题。

---

### 性能与权重
- 目前演示版使用固定权重（编译会有警告）。建议后续：
  - 启用 `runtime-benchmarks`，为所有调用生成权重；
  - 在 `Config` 中接入 `WeightInfo`，各 extrinsic/内部调用按基准权重标注。

---

### 使用与集成指引
1) 在 runtime 中完成 `pallet-karma` 与 `pallet-authorizer` 的依赖、注册与配置（包括 `KarmaNsBytes` 命名空间）。
2) 通过 `pallet-authorizer` 提案把业务模块账户加入 Karma 白名单。
3) 业务 Pallet 内部使用 `KarmaCurrency::gain/spend` 完成积分获取与消费；`memo` 建议使用简短、可审计的元信息（如来源、任务 ID）。
4) 前端/脚本可读取：`KarmaOf`（余额）、`TotalMeritOf`（总功德）、`LevelOf`（等级）、`TotalBurned`（全网黑洞）、`HistoryOf`（历史）。

---

### 安全与边界
- 非经济化与不可转移：杜绝投机与滥用。
- 授权中心统一治理：降低各 Pallet 自行维护白名单的复杂度与治理成本。
- 审计友好：事件 + 历史存储 + 黑洞统计，便于链上追溯与看板展示。

---

### 未来扩展
- 授权细粒度化：支持 `(Namespace, Operation, AccountId)` 维度的授权。
- 等级曲线参数化：支持治理更改阈值或使用查表曲线。
- 视图与 RPC：提供只读 RPC 以便前端快速拉取等级、功德与最近 N 次历史。
- 权重优化：全量基准测试与链上性能调优。