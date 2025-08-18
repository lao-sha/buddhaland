# pallet-exchange

## 概述
- 实现 BUD → Karma 的单向兑换，且支持动态的“PalletId 派生模块子账户分配（BPS）”。
- 分配管理走 `pallet-authorizer` 的命名空间白名单；仅授权账户可创建/更新/删除/暂停分配项。
- 兑换不走会话代付，必须用户自行签名与付费。

## 核心逻辑
1. 分配项以 `[u8;8]` 为键，记录 `bps` 与 `enabled`；所有启用项 BPS 总和必须等于 `BpsDenominator`（默认 10000）。
2. 兑换时：
   - 将用户输入的 BUD `amount` 按 BPS 拆分为多份，逐份划转到 `PalletId.into_sub_account_truncating(id)` 派生的子账户。
   - 尾差（整除误差）补给第一位启用的分配项，确保总额严格等于 `amount`。
   - 由模块主账户（`PalletId.into_account_truncating()`）作为调用者，调用 `pallet-karma::gain` 为用户增发等额 Karma。

## 主要接口
- `set_allocs(items)`：批量替换分配项（覆盖式），要求启用项 BPS 之和 == `BpsDenominator`。
- `update_alloc(id, bps, enabled)`：更新单项，更新后会全量校验 BPS 和。
- `remove_alloc(id)`：删除分配项，删除后会全量校验 BPS 和。
- `exchange(amount, memo)`：用户发起兑换并自动分配划转与铸发 Karma。

## Config
- `type Currency`：BUD 代币。
- `type PalletIdGet`：用于派生模块主账户与子账户。
- `type BpsDenominator`：BPS 分母（默认 10000）。
- `type MaxAllocs`：分配项上限；`type MaxMemoLen`：备注上限。
- `type AdminAuthorizerNs`：管理命名空间（仅管理接口校验，不用于代付）。
- `type WeightInfo`：基准权重类型。

## 安全
- 严格校验启用项 BPS 总和；金额为零拒绝。
- 兑换前再次校验 BPS 和，防止配置被篡改后导致划转异常。
- Exchange 的 `exchange` 未加入 forwarder 白名单，必须自签自付。
