# pallet-order

## 概述
- 管理寺庙服务订单：下单、代办接单、开始执行、凭证提交（图片/视频 CID，IPFS）、买家确认、超时结算、仲裁接入。
- 资金流由 `pallet-escrow` 托管，释放/退款由订单与仲裁驱动。

## 核心流程
1. `create_order`：买家下单，调用 `Escrow::lock_from` 托管入金（状态 `Created`）。
2. `accept_order`：代办接单，绑定代办帐号（状态 `Assigned`）。
3. `start_order`：代办开始执行（状态 `InProgress`）。
4. `submit_order_proof`：代办提交图片/视频 CID（上限：20 图/5 视频），进入买家确认期（状态 `Submitted`）。
5. `confirm_done_by_buyer`：买家确认完成，放款给代办并平台分账；随后按订单金额 1:1 通过本地适配器调用 Karma 模块为买家增发 Karma（状态 `Closed`）。
6. `finalize_expired`：到期未争议自动放款（状态 `Closed`）。
7. 仲裁：`arbitrate_release/refund/partial` 由仲裁路由触发（状态至 `Closed`）。

状态机：`Created → Assigned → InProgress → Submitted → Released/Refunded → Closed`

## 仲裁接入
- 实现 `ArbitrationOrderHook`：
  - `arbitrate_release`/`arbitrate_refund`/`arbitrate_partial(bps)` 调用 `Escrow` 完成资金释放或退款。
- 由 `pallet-arbitration` 的 Router 路由到本 Pallet 的 Hook。

## Extrinsics
- `create_order(temple_id, service_id, qty, locked)`：下单并托管入金。
- `accept_order(id)`：代办接单，要求 `status == Created` 且未绑定代办。
- `start_order(id)`：代办开始执行，要求调用者为订单代办且 `status == Assigned`。
- `submit_order_proof(id, imgs, vids, note_hash)`：仅订单代办可提交，要求 `status == InProgress`。
- `confirm_done_by_buyer(id)`：买家确认放款并分账。
- `finalize_expired(id)`：任何人触发超时自动放款。

### 事件说明
- `OrderAccepted { id }`：代办接单。
- `OrderStarted { id }`：代办开始执行。
- `ProofSubmitted { id, img_count, vid_count }`：其中 `img_count/vid_count` 为本次提交的实际数量（非配置上限）。

## Config
- `type Currency`、`type PalletIdGet`（订单子账户）、`type PlatformAccount`、`type PlatformFeeBps`。
- `type Escrow`：托管接口实现。
- `type Karma`：本地适配 Trait（`KarmaMint`）。Runtime 通过适配器（示例：`KarmaMintAdapter`）桥接到 `pallet-karma` 的 `KarmaCurrency::gain`，用于完成态 1:1 增发。
- `type WeightInfo`：基准权重类型。
- 各项限值：`MaxCidLen`、`MaxImg`、`MaxVid`、`ConfirmTTL`。

## 权重
- 已接入基准框架（含 `accept_order`/`start_order`），运行后替换为 `weights::SubstrateWeight<Runtime>`。

## 代付白名单
- 订单关键 Extrinsic 已纳入 Forwarder 命名空间（详见 runtime 配置）。
- 注意：兑换 `pallet-exchange::exchange` 未纳入代付白名单，需用户自签自付（与订单无关）。
