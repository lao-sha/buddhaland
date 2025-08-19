# pallet-agent

## 概述
- 管理代办人（僧尼/服务执行者）资料、技能、可预约时段等，服务于寺庙订单履约。
- 代办人与寺庙多对多绑定：`(agent_id, temple_id) -> Status(Pending/Active/Revoked)`（建议在本 pallet 承载）。
- 代办项目与规格（SKU）与寺庙服务目录对齐：每个项目显式指向 `(temple_id, service_id)`，多规格/多价格；媒体仅存 CID（图≤10、视≤2、文≤5）。

## 能力
- 注册/更新代办资料与技能列表（受上限约束）。
- 维护可接单时段（日程）。
- 绑定寺庙与项目发布：仅在绑定 Active 且服务启用时允许发布；下线/解绑联动 `unpublish`。
- 列表/详情只读接口（Trait）供 `pallet-order` 使用：按 `(temple_id, service_id)` 列出可服务代办人、读取 SKU 价格快照等。

## Config
- `type Currency`：押金或费用（MVP 可不启用）。
- `type MaxSkills`、`type MaxCalendar`：上限参数。

## 集成
- 与 `pallet-temple` 的服务目录和 `pallet-order` 的履约流程配合。
- 订单状态节点可通过统计 Trait（如 `AgentStatsWriter`）回写“成单量/成交额/好评/差评/售后”等计数。

## 安全
- 使用 `BoundedVec` 限制可变长字段；可结合 `pallet-authorizer` 做更细的准入控制。
- 名称一致性：服务名以 `pallet-temple` 为单一事实源，代办项目仅引用 `(temple_id, service_id)`，可用 `subtitle/tagline` 补充说明。
