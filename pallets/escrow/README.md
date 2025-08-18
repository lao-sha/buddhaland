# pallet-escrow

## 概述
- 通用托管账户管理：锁定、释放、退款、转移，面向多业务复用（订单、OTC、仲裁）。
- 使用 `PalletId` 派生托管主账户与子账户（按业务对象 ID）。

## 接口（Trait）
- `lock_from(buyer, id, amount)`：从 `buyer` 锁定 `amount` 到 `id` 对应的托管子账户。
- `transfer_from_escrow(id, to, amount)`：从托管子账户转出到 `to`。
- `release_all(id, to)`：释放该子账户全部资金到 `to`。
- `refund_all(id, to)`：退款全部资金到 `to`。
- `amount_of(id) -> Balance`：查询托管子账户余额。

## Config
- `type Currency`：底层代币实现（BUD）。
- `type EscrowPalletId`：派生模块账户。

## 用例
- 订单履约结算：由订单或仲裁调用，释放给代办或退还买家。
- OTC 卖单保证金：下单锁定、取消退款。

## 安全
- 仅受信 Pallet 通过约定流程调用（通过 runtime 绑定接口）。
