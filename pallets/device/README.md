# pallet-device

## 概述
- 管理冥想头环设备：注册、唯一性分级（弱/中/强）、账户绑定/解绑与挑战流程。
- 仅存储哈希与证书锚点等摘要信息，保护隐私。

## 设计要点
- 唯一性分级可升级：弱唯一（哈希）→ 中/强唯一（设备公钥 + 签名挑战）。
- 绑定流程：`open_bind_challenge` → 设备签名/证明 → `bind_headband` 完成绑定。
- 与 `pallet-meditation` 协同：提交会话摘要前可校验设备登记与绑定状态。

## Config
- `type AdminOrigin`: 管理接口权限。
- `type Currency`: 押金等（MVP 可为 0）。
- `type MaxDevicesPerOwner`, `type ChallengeTtl`, `type MinRegisterDeposit`, `type MaxMetaLen`。
- `type WeightInfo`：提供基准权重。

## 主要 Extrinsic（MVP）
- `register_headband(...)`：登记设备（存储哈希/公钥/元数据）。
- `open_bind_challenge(device_id)`：开启绑定挑战，TTL 内有效。
- `bind_headband(device_id, proof)`：验证通过后绑定到账户。
- `unbind_headband(device_id)`：解绑。

## 事件与错误
- 事件字段使用 `u8` 等标量避免 `DecodeWithMemTracking` 约束。
- 常见错误：设备已存在、挑战过期、绑定冲突等。

## 权重
- 已接入 `T::WeightInfo` 占位，需后续生成 `weights.rs` 替换。
