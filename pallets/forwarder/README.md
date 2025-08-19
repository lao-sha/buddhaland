# pallet-forwarder

## 概述
- 实现“元交易转发 + 会话许可 + 赞助者代付”，允许用户线下签名，赞助者代为上链并付费。
- 通过 `pallet-authorizer` 控制赞助者白名单与可转发调用范围（命名空间隔离）。

## 核心能力
- 会话许可：TTL、命名空间、范围、nonce/上限，用户单次签名建立会话。
- 元交易：外层由赞助者签名付费，内层以用户身份执行 RuntimeCall。
- 反逃逸：`ForbiddenCalls` 禁用如 `Sudo` 等高权限或逃逸调用。

## 关键设计
- 为避免 `RuntimeCall` 类型循环与 `DecodeWithMemTracking` 约束，Extrinsic 参数使用 `BoundedVec<u8>` 承载 SCALE 编码后的 `SessionPermit` 与 `MetaTx`，在链上内部解码。
- `Config`:
  - `type RuntimeCall`、`type Authorizer`、`type ForbiddenCalls`、`type MaxMetaLen`、`type MaxPermitLen`。

## 主要 Extrinsic（示意）
- `open_session(permit_bytes: BoundedVec<u8>)`
- `forward(meta_bytes: BoundedVec<u8>)`

## 集成与白名单
- 在 `runtime/src/configs/mod.rs` 的 `AuthorizerAdapter::is_call_allowed` 匹配允许代付的 RuntimeCall 变体（按命名空间）。
- Exchange 的 `exchange` 未纳入白名单，必须用户自行签名与付费。
- 仲裁 `dispute/arbitrate` 可纳入特定域的白名单，由 `pallet-authorizer` 管控仲裁方/路由权限。
