# pallet-mining

## 概述
- 基于冥想会话质量发放 BUD 奖励（或积分），具备日上限/设备上限/全局上限与反刷机制。
- 对外提供 `MiningInterface` 以便 `pallet-meditation` 触发奖励。

## 设计要点
- 授权适配：`MiningAuthorizer` 通过 `pallet-authorizer` 校验调用者。
- 参数：`MaxSessionMinutes`、多级日上限、`BaseBudPerMinute` 等。
- 安全计算：使用 `Saturating` 加法乘法。

## Extrinsic
- `mine(valid: bool, valid_minutes: u16, quality_pct: u8)`：用户直接上报（MVP）。

## 内部接口
- `award_by(who, minutes, quality)`：由 `pallet-meditation` 调用的核心奖励逻辑。

## 集成
- `runtime/src/configs/mod.rs` 实现 `MiningAuthorizerAdapter`，桥接到 `pallet-authorizer` 命名空间。

## 权重
- 已接入 `T::WeightInfo` 占位；请运行基准后替换为 `SubstrateWeight<Runtime>`。
