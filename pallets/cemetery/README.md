pallet-cemetery
===============

管理陵园与墓位（单人/合葬），记录安葬/迁出事件。管理员可设置为多签账户（复用 pallet-multisig）。

- 安葬成功触发 `IntermentCommitted` 事件，可用于订单自动验收或自动授权逝者编辑者。
- 与 `pallet-deceased` 低耦合，通过可选回调 Trait 通知。


