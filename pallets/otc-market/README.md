# pallet-otc-market

## 概述
- BUD 主网代币的链上 OTC 场外交易撮合（MVP：订单登记与取消）。
- 卖单资金托管到 `pallet-escrow`，取消时原路退款，成交由业务扩展负责。

## 核心能力
- `place_order`：登记买/卖订单；卖单调用 `Escrow::lock_from` 托管资金。
- `cancel_order`：卖单取消时调用 `Escrow::refund_all`。

## Config
- `type Balance`：币种精度与类型。
- `type Escrow`：托管接口。
- `type MaxNotesLen`：订单备注上限。
- `type WeightInfo`：基准权重。

## 权重
- 已接入基准框架，运行后替换为 `weights::SubstrateWeight<Runtime>`。

## 安全
- 使用 `BoundedVec` 限制可变长字段，防止存储膨胀。
- 复杂撮合与清结算可在后续版本迭代。
