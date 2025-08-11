# Exchange Pallet

BUD代币兑换Karma（福缘值）的pallet。实现单向兑换机制，并将兑换的BUD按比例分配到三个目标。

## 功能概述

Exchange Pallet专门处理BUD代币到Karma的单向兑换，支持将收到的BUD按照预设比例分配到不同的账户：
- 一部分销毁（黑洞）
- 一部分进入国库
- 一部分进入paymaster用于支付用户gas费

## 核心功能特性

### 1. 单向兑换
- **BUD → Karma**: 支持用户使用BUD代币兑换Karma福缘值
- **不可逆**: Karma不能兑换回BUD，确保福缘值的精神价值
- **可配置比例**: 兑换比例通过runtime配置项设定

### 2. BUD分配机制
兑换得到的BUD将按比例分配到三个目标：
- **黑洞 (Burn)**: 永久销毁，减少代币流通量（默认20%）
- **国库 (Treasury)**: 用于生态发展和治理（默认70%）
- **Paymaster**: 用于代付用户交易费用（默认10%）

### 3. 配置参数
- `ExchangeRate`: 兑换比例（1 BUD = X Karma）
- `BurnBps`: 黑洞分配比例（基点制，10000=100%）
- `TreasuryBps`: 国库分配比例
- `PaymasterBps`: Paymaster分配比例
- 账户配置：`BlackholeAccount`, `TreasuryAccount`, `PaymasterAccount`

## 设计原则

### 安全性
- **原子性**: 兑换过程要么全部成功，要么全部失败
- **比例验证**: 确保三个分配比例之和等于100%
- **余额检查**: 兑换前检查用户BUD余额是否充足
- **错误处理**: 详细的错误类型和回滚机制

### 透明性
- **事件记录**: 每次兑换都记录详细事件
- **可追溯**: 所有分配去向都在链上可查
- **公开配置**: 所有参数都是公开的常量

### 可配置性
- **灵活比例**: 支持runtime调整分配比例
- **可更新账户**: 支持更换目标账户地址
- **动态汇率**: 支持调整兑换比例

## 存储设计

本pallet为无状态设计，不需要额外存储，所有操作都是即时执行。

## 事件

```rust
/// 成功兑换Karma [用户, bud_in, karma_out, burn, treasury, paymaster]
Exchanged(AccountId, u128, KarmaBalance, u128, u128, u128)
```

## 错误类型

```rust
pub enum Error<T> {
    /// 输入金额为0
    ZeroAmount,
    /// 分配比例之和不等于BpsDenominator
    InvalidBps,
    /// 代币转账失败
    TransferFailed,
    /// Karma发放失败
    KarmaMintFailed,
}
```

## 外部调用 (Extrinsics)

### exchange(amount: u128)
用户调用此函数用BUD兑换Karma。

**参数**:
- `amount`: 要兑换的BUD数量

**逻辑**:
1. 验证输入金额 > 0
2. 验证分配比例配置正确
3. 计算三个目标的分配金额
4. 依次转账到黑洞、国库、paymaster
5. 给用户发放对应数量的Karma
6. 发出Exchanged事件

## 与其他Pallet的集成

### 依赖关系
- **pallet-balances**: 处理BUD代币转账
- **pallet-karma**: 发放Karma福缘值
- **pallet-paymaster**: 接收BUD用于代付gas费

### 配置示例

```rust
impl pallet_exchange::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type PaymasterAccount = PaymasterAccountId;
    type BlackholeAccount = BlackholeAccountId; 
    type TreasuryAccount = TreasuryAccountId;
    type ExchangeRate = ConstU128<1000>; // 1 BUD = 1000 Karma
    type BurnBps = ConstU32<2000>;       // 20%
    type TreasuryBps = ConstU32<7000>;   // 70%
    type PaymasterBps = ConstU32<1000>;  // 10%
    type BpsDenominator = ConstU32<10000>; // 10000基点 = 100%
}
```

## 使用场景

1. **用户体验提升**: 用户可以直接用BUD购买Karma，参与佛境生态
2. **经济闭环**: 通过销毁机制控制代币通胀
3. **生态发展**: 国库资金用于生态建设
4. **降低门槛**: Paymaster资金池降低用户交易门槛

## 安全考虑

- **不可逆性**: 确保Karma不能兑换回BUD
- **账户安全**: 黑洞账户应该是不可控制的销毁地址
- **权限控制**: 只有用户自己可以发起兑换
- **余额保护**: 防止恶意消耗用户余额

## 测试建议

1. **单元测试**: 测试各种兑换金额和分配计算
2. **集成测试**: 测试与karma和balances pallet的交互
3. **边界测试**: 测试零金额、最大金额等边界情况
4. **错误测试**: 测试各种错误场景的处理