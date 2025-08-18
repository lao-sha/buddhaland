# pallet-agent

## 概述
- 管理代办人（僧尼/服务执行者）资料、技能、可预约时段等，服务于寺庙订单履约。

## 能力
- 注册/更新代办资料与技能列表（受上限约束）。
- 维护可接单时段（日程）。

## Config
- `type Currency`：押金或费用（MVP 可不启用）。
- `type MaxSkills`、`type MaxCalendar`：上限参数。

## 集成
- 与 `pallet-temple` 的服务目录和 `pallet-order` 的履约流程配合。

## 安全
- 使用 `BoundedVec` 限制可变长字段；可结合 `pallet-authorizer` 做更细的准入控制。
