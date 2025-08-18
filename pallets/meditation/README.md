# pallet-meditation

## 概述
- 上链冥想会话“摘要/承诺”（不含原始脑波数据）。
- 可选验证：设备是否登记、是否绑定、是否带设备签名。
- 成功记录后调用 `pallet-mining` 发放奖励。

## 设计要点
- 为规避复杂泛型在 Extrinsic 参数上的解码约束，`submit_session` 接受 `BoundedVec<u8>`（SCALE 编码的 header），链上解码。
- `Config`：`MaxOffchainLen`、`RequireHeadband`、`RequireBinding`、`RequireDeviceSignature`、`MaxHeaderLen`。
- 与 `pallet-device`、`pallet-mining` 解耦，通过 `Config` 扩展。

## Extrinsic
- `submit_session(header_bytes: BoundedVec<u8, MaxHeaderLen>)`：提交会话摘要并触发奖励。

## 权重
- 已接入 `T::WeightInfo` 占位；请运行基准后替换。
