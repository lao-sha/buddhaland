# pallet-arbitration

## 概述
- 仲裁中枢：统一登记争议与执行裁决，采用“域 + 路由 + 钩子 + 托管”的低耦合架构。
- 面向多业务来源（订单、OTC 等）统一处理，具体资金流由业务 Hook 实现。

## 架构
- Domain: `[u8;8]` 标识业务域（如订单域）。
- Router: 运行时实现 `ArbitrationRouter`，将 `dispute/arbitrate` 路由到对应业务。
- Hook: 业务 Pallet 实现 `ArbitrationOrderHook`（或各自域的 Hook）执行释放/退款/部分裁决。
- Escrow: 通过 `pallet-escrow` 完成资金释放/退款。

## Extrinsic（MVP）
- `dispute(domain, id, reason)`：登记争议，校验是否允许进入仲裁（由 Router 的 `can_dispute` 决定）。
- `arbitrate(domain, id, decision)`：执行裁决，Router 调用业务 Hook 落地资金流。

## Config
- `type Router`：域路由实现。
- `type Escrow`：托管接口。
- `type MaxEvidence`, `type MaxCidLen`：证据及 CID 限制。
- `type WeightInfo`：基准权重。

## 集成
- 订单域：Router 将裁决映射到 `pallet-order` 的 `ArbitrationOrderHook`，调用 `Escrow` 实施资金流。

## 权重
- 已接入基准框架，运行后替换为 `weights::SubstrateWeight<Runtime>`。

## 会话代付
- `dispute/arbitrate` 已纳入 Forwarder 白名单命名空间（开发网示例）；可在 Authorizer 控制仲裁者白名单。
