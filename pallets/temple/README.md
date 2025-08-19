# pallet-temple

## 概述
- 在链上登记寺庙与其服务清单（如：供灯/供花/供果/供香/放生/供僧/建寺/添油/印经等），仅存储哈希与必要元数据，保护隐私与节省链上空间。
- temple 仅提供目录与文案，不包含任何价格信息（定价权归属 `pallet-agent`）。
- 面向上层的下单履约流程（`pallet-order`）提供只读目录与校验依据。

## 数据结构
- `Temple { owner, name_hash, geo_hash, profile_hash, active }`
  - 仅存摘要哈希（比如 IPFS CID 的哈希或 keccak/blake2 哈希），不存明文。
- `Service { kind, title_hash, desc_hash, active }`
  - `kind: ServiceKind`（枚举：Light/Flower/Fruit/Incense/Release/Monk/Build/Oil/Sutra）。
- `CalendarSlot { date_block, lunar_tag }`：公历日期近似为 `date_block`（区块高度），附带农历标签。

主存储：
- `Temples: TempleId => Temple`
- `Services: (TempleId, ServiceId) => Service`
- `Calendars: ((TempleId, ServiceId), month_key) => BoundedVec<CalendarSlot>`
- `NextTempleId: TempleId`；`NextServiceId: TempleId => ServiceId`

## Extrinsics
- `register_temple(name_hash, geo_hash, profile_hash)`
  - 由任意账户注册寺庙，记录所有者为调用者，初始 `active=true`。
- `add_service(temple, kind_code, title_hash, desc_hash)`
  - 仅寺庙所有者可调用；`kind_code: u8` 映射到 `ServiceKind`，避免自定义枚举在 Call 参数的解码问题。
  - 将服务 `active=true` 并分配自增 `ServiceId`。

注意：当前最小实现尚未提供更新/上下架/日历增改等接口，可按需扩展（保持低耦合，避免在此 pallet 内引入业务流程）。

## 事件
- `TempleRegistered { id }`：新寺庙注册。
- `ServiceAdded { temple, service }`：新服务添加。
- `CalendarUpdated { temple, service }`：日历更新（预留）。

## 错误
- `TempleNotFound`：寺庙不存在。
- `ServiceNotFound`：服务不存在。
- `NotOwner`：非寺庙所有者尝试操作。
- `InvalidKind`：非法服务类型码。

## Config
- `type MaxCalendar: Get<u32>`：单月日历槽位最大数量。
- `type Balance`：保留为资金相关 pallet 的类型对齐（本 pallet 不再直接使用价格字段）。

## 与其它 Pallet 的关系
- `pallet-order`：读取 `Temple/Service` 元数据，作为下单时的目录来源。
- `pallet-agent`：维护实际的代办定价与版本；代办项目必须显式指向 `(temple_id, service_id)`，服务名以本 Pallet 为唯一事实源。
- `pallet-authorizer`：可用于限制谁可注册寺庙/维护目录（通过命名空间白名单）。

## 安全与设计原则
- 仅上链哈希/摘要，不上链明文，避免可跟踪性与隐私泄露。
- 所有可变长字段使用 `BoundedVec`，防止存储膨胀与权重不确定性。
- 保持职责单一：temple 只负责目录与元数据，不直接参与资金/订单/履约流程。
- 通过 trait 适配器与 runtime 配置实现低耦合集成。
