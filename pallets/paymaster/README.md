# Paymaster Pallet

代理支付gas费用的pallet，为用户提供无缝的交易体验，特别适用于需要频繁交易的DeFi应用场景。

## 功能概述

Paymaster Pallet 是一个专门设计用于处理代理支付交易费用的模块，允许指定的代理账户代替用户支付gas费用，从而降低用户使用门槛，提升用户体验。

### 系统托管池
- **集中资金管理**: 通过系统托管池统一管理来自exchange pallet等外部模块的资金
- **免签名代付**: 支持使用托管池资金进行免签名的代理支付
- **资金来源**: 主要通过exchange pallet的BUD兑换收入和其他生态模块注入资金
- **透明追踪**: 所有托管池资金变动都有完整的事件记录

## 核心功能特性

### 1. 预付费系统
- **充值机制**: 用户可预先充值资金到paymaster账户
- **第三方代付**: 支持任何账户为其他用户充值预付费余额
- **批量代付**: 支持一次性为多个用户充值
- **余额管理**: 实时跟踪用户预付费余额
- **最小充值限制**: 防止小额充值造成的资源浪费
- **安全提取**: 支持用户随时提取未使用的预付费余额

### 2. 授权充值系统
- **预授权机制**: 用户可预先授权特定账户为自己充值
- **金额限制**: 可设置单次充值的最大金额限制
- **灵活控制**: 支持随时启用/禁用授权
- **安全保障**: 防止未经同意的代付行为

### 3. 代理权限管理
- **细粒度权限控制**: 支持多种权限类型
  - `All`: 完全权限，可执行所有交易
  - `Transfer`: 仅限转账操作
  - `Governance`: 仅限治理相关操作
  - `Custom`: 自定义权限，指定特定pallet调用
- **费用限额**: 为每个代理设置单次交易费用上限
- **启用/禁用**: 灵活控制代理状态

### 4. 系统托管池管理
- **外部资金注入**: 支持其他pallet（如exchange）向托管池注入资金
- **免签名代付**: 使用托管池资金进行无需用户签名的代理支付
- **资金统计**: 实时跟踪托管池总额和使用情况
- **安全控制**: 严格的权限控制确保资金安全

### 批量交易处理
- **批量执行**: 支持一次性执行多个交易，降低总体费用
- **数量限制**: 可配置的最大批量交易数量
- **原子性**: 批量交易要么全部成功，要么全部失败

### 5. 费用追踪与统计
- **详细记录**: 记录每笔交易的费用消耗
- **第三方支付记录**: 完整追踪第三方代付历史
- **历史查询**: 支持查询用户的费用使用历史
- **服务费收取**: 可配置的服务费率，为运营方提供收入
- **透明计费**: 所有费用计算过程完全透明

## 配置参数

### Pallet配置
```rust
pub trait Config: frame_system::Config {
    /// 运行时事件类型
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    
    /// 货币类型（通常为原生代币）
    type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
    
    /// Pallet ID，用于生成模块账户
    type PalletId: Get<PalletId>;
    
    /// 最大批量交易数量
    type MaxBatchSize: Get<u32>;
    
    /// 最小预付费金额
    type MinimumDeposit: Get<BalanceOf<Self>>;
    
    /// 服务费率（百分比）
    type ServiceFeeRate: Get<Percent>;
    
    /// 权重信息
    type WeightInfo: WeightInfo;
}
```

### 运行时配置示例
```rust
parameter_types! {
    pub const PaymasterPalletId: PalletId = PalletId(*b"paymastr");
    pub const MaxBatchSize: u32 = 50;
    pub const MinimumDeposit: Balance = 1000 * UNIT;
    pub const ServiceFeeRate: Percent = Percent::from_percent(1); // 1% 服务费
}

impl pallet_paymaster::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type PalletId = PaymasterPalletId;
    type MaxBatchSize = MaxBatchSize;
    type MinimumDeposit = MinimumDeposit;
    type ServiceFeeRate = ServiceFeeRate;
    type WeightInfo = pallet_paymaster::weights::SubstrateWeight<Runtime>;
}
```

## 存储结构

### 预付费余额
```rust
/// 用户预付费余额映射
PrepaidBalances: StorageMap<AccountId, Balance>
```

### 代理配置
```rust
/// 代理配置双重映射 (用户账户 -> 代理账户 -> 配置)
ProxyConfigs: StorageDoubleMap<AccountId, AccountId, ProxyConfig>

/// 代理配置结构
pub struct ProxyConfig<AccountId, Balance> {
    pub proxy: AccountId,           // 代理账户
    pub permission: ProxyPermission, // 权限类型
    pub fee_limit: Balance,         // 费用限额
    pub enabled: bool,              // 是否启用
}
```

### 第三方支付相关存储
```rust
/// 第三方支付记录 (支付方 -> 受益人 -> 记录列表)
ThirdPartyPayments: StorageDoubleMap<AccountId, AccountId, BoundedVec<ThirdPartyPaymentRecord, 50>>

/// 充值授权配置 (受益人 -> 授权支付方 -> 授权配置)
DepositAuthorizations: StorageDoubleMap<AccountId, AccountId, DepositAuthorization>

/// 第三方支付记录结构
pub struct ThirdPartyPaymentRecord<Balance, BlockNumber> {
    pub amount: Balance,            // 支付金额
    pub block_number: BlockNumber,  // 区块高度
    pub timestamp: u64,             // 时间戳
}

/// 充值授权结构
pub struct DepositAuthorization<Balance> {
    pub max_amount: Option<Balance>, // 最大授权金额
    pub enabled: bool,               // 是否启用
}
```

### 费用记录
```rust
/// 费用使用记录（每用户最多保存100条）
FeeRecords: StorageMap<AccountId, BoundedVec<FeeRecord, 100>>

/// 费用记录结构
pub struct FeeRecord<Balance, BlockNumber> {
    pub amount: Balance,            // 消费金额
    pub block_number: BlockNumber,  // 区块高度
    pub tx_hash: Option<[u8; 32]>,  // 交易哈希
}
```

### 服务费统计
```rust
/// 总服务费收入
TotalServiceFees: StorageValue<Balance>
```

## 主要功能调用

### 1. 预付费管理

#### 自主充值预付费
```rust
/// 用户充值预付费到paymaster账户
pub fn deposit_prepaid(
    origin: OriginFor<T>,
    amount: BalanceOf<T>,
) -> DispatchResult
```

**参数说明**:
- `origin`: 用户签名来源
- `amount`: 充值金额（必须大于等于最小充值金额）

**使用示例**:
```rust
// 充值 10,000 个代币作为预付费
pallet_paymaster::deposit_prepaid(origin, 10000 * UNIT)?;
```

#### 第三方代付充值
```rust
/// 第三方为用户充值预付费
pub fn deposit_prepaid_for(
    origin: OriginFor<T>,
    beneficiary: T::AccountId,
    amount: BalanceOf<T>,
) -> DispatchResult
```

**参数说明**:
- `origin`: 支付方签名来源
- `beneficiary`: 受益人账户
- `amount`: 充值金额

**使用示例**:
```rust
// 企业为员工充值 5,000 个代币
pallet_paymaster::deposit_prepaid_for(
    company_origin,
    employee_account,
    5000 * UNIT,
)?;
```

#### 批量第三方代付
```rust
/// 批量为多个用户充值预付费
pub fn batch_deposit_prepaid_for(
    origin: OriginFor<T>,
    deposits: Vec<(T::AccountId, BalanceOf<T>)>,
) -> DispatchResult
```

**参数说明**:
- `origin`: 支付方签名来源
- `deposits`: 充值列表 (受益人, 金额)

**使用示例**:
```rust
// 批量为多个员工充值
let deposits = vec![
    (employee1, 1000 * UNIT),
    (employee2, 1500 * UNIT),
    (employee3, 2000 * UNIT),
];

pallet_paymaster::batch_deposit_prepaid_for(
    company_origin,
    deposits,
)?;
```

#### 提取预付费
```rust
/// 用户提取未使用的预付费余额
pub fn withdraw_prepaid(
    origin: OriginFor<T>,
    amount: BalanceOf<T>,
) -> DispatchResult
```

**参数说明**:
- `origin`: 用户签名来源
- `amount`: 提取金额（不能超过当前余额）

**使用示例**:
```rust
// 提取 5,000 个代币
pallet_paymaster::withdraw_prepaid(origin, 5000 * UNIT)?;
```

### 2. 授权充值管理

#### 设置充值授权
```rust
/// 设置充值授权，允许特定账户为自己充值
pub fn set_deposit_authorization(
    origin: OriginFor<T>,
    authorized_payer: T::AccountId,
    max_amount: Option<BalanceOf<T>>,
    enabled: bool,
) -> DispatchResult
```

**参数说明**:
- `origin`: 受益人签名来源
- `authorized_payer`: 授权的支付方账户
- `max_amount`: 最大授权金额（None表示无限制）
- `enabled`: 是否启用授权

**使用示例**:
```rust
// 授权特定账户为自己充值，最大金额10,000代币
pallet_paymaster::set_deposit_authorization(
    user_origin,
    trusted_payer,
    Some(10000 * UNIT),
    true,
)?;

// 禁用授权
pallet_paymaster::set_deposit_authorization(
    user_origin,
    trusted_payer,
    None,
    false,
)?;
```

#### 授权充值
```rust
/// 使用预先授权进行充值
pub fn authorized_deposit_prepaid(
    origin: OriginFor<T>,
    beneficiary: T::AccountId,
    amount: BalanceOf<T>,
) -> DispatchResult
```

**参数说明**:
- `origin`: 授权支付方签名来源
- `beneficiary`: 受益人账户
- `amount`: 充值金额

**使用示例**:
```rust
// 使用预先授权为用户充值
pallet_paymaster::authorized_deposit_prepaid(
    trusted_payer_origin,
    user_account,
    3000 * UNIT,
)?;
```

### 3. 代理管理

#### 添加代理
```rust
/// 为用户添加代理账户
pub fn add_proxy(
    origin: OriginFor<T>,
    proxy: T::AccountId,
    permission: ProxyPermission,
    fee_limit: BalanceOf<T>,
) -> DispatchResult
```

**参数说明**:
- `origin`: 用户签名来源
- `proxy`: 代理账户地址
- `permission`: 代理权限类型
- `fee_limit`: 单次交易费用限额

**使用示例**:
```rust
// 添加转账代理，费用限额 1,000 代币
pallet_paymaster::add_proxy(
    origin,
    proxy_account,
    ProxyPermission::Transfer,
    1000 * UNIT,
)?;
```

#### 移除代理
```rust
/// 移除指定的代理账户
pub fn remove_proxy(
    origin: OriginFor<T>,
    proxy: T::AccountId,
) -> DispatchResult
```

**使用示例**:
```rust
// 移除代理账户
pallet_paymaster::remove_proxy(origin, proxy_account)?;
```

### 4. 代理执行

#### 单笔代理执行
```rust
/// 代理执行单笔交易
pub fn proxy_execute(
    origin: OriginFor<T>,
    user: T::AccountId,
    call: Box<<T as Config>::RuntimeCall>,
    estimated_fee: BalanceOf<T>,
) -> DispatchResultWithPostInfo
```

**参数说明**:
- `origin`: 代理账户签名来源
- `user`: 目标用户账户
- `call`: 要执行的交易调用
- `estimated_fee`: 预估交易费用

**使用示例**:
```rust
// 代理执行转账交易
let call = Box::new(RuntimeCall::Balances(pallet_balances::Call::transfer {
    dest: dest_account,
    value: 100 * UNIT,
}));

pallet_paymaster::proxy_execute(
    proxy_origin,
    user_account,
    call,
    estimated_fee,
)?;
```

#### 批量代理执行
```rust
/// 代理批量执行多笔交易
pub fn batch_proxy_execute(
    origin: OriginFor<T>,
    user: T::AccountId,
    calls: Vec<Box<<T as Config>::RuntimeCall>>,
    estimated_total_fee: BalanceOf<T>,
) -> DispatchResultWithPostInfo
```

**参数说明**:
- `origin`: 代理账户签名来源
- `user`: 目标用户账户
- `calls`: 要执行的交易调用列表
- `estimated_total_fee`: 预估总交易费用

**使用示例**:
```rust
// 批量执行多笔转账
let calls = vec![
    Box::new(RuntimeCall::Balances(pallet_balances::Call::transfer {
        dest: dest1,
        value: 100 * UNIT,
    })),
    Box::new(RuntimeCall::Balances(pallet_balances::Call::transfer {
        dest: dest2,
        value: 200 * UNIT,
    })),
];

pallet_paymaster::batch_proxy_execute(
    proxy_origin,
    user_account,
    calls,
    estimated_total_fee,
)?;
```

## 权限类型详解

### ProxyPermission 枚举
```rust
pub enum ProxyPermission {
    /// 完全权限 - 可执行所有类型的交易
    All,
    
    /// 仅转账权限 - 只能执行余额转账操作
    Transfer,
    
    /// 仅治理权限 - 只能执行治理相关操作
    Governance,
    
    /// 自定义权限 - 指定特定的pallet调用
    Custom(Vec<u8>),
}
```

### 权限验证机制
代理执行交易时，系统会根据配置的权限类型验证是否允许执行特定操作：

1. **All权限**: 无限制，可执行任何交易
2. **Transfer权限**: 仅允许余额转账相关操作
3. **Governance权限**: 仅允许治理投票、提案等操作
4. **Custom权限**: 根据自定义规则验证

## 费用计算机制

### 费用组成
每笔代理交易的总费用包括：

1. **基础交易费用**: 区块链网络的标准交易费用
2. **服务费**: 按配置的服务费率计算的额外费用

### 计算公式

### 费用扣除流程
1. 验证用户预付费余额是否充足
2. 从用户预付费余额中扣除总费用
3. 将服务费计入系统总收入
4. 记录费用使用历史

## 事件系统

### 主要事件类型
```rust
pub enum Event<T: Config> {
    PrepaidDeposited { user: T::AccountId, amount: BalanceOf<T> },
    PrepaidWithdrawn { user: T::AccountId, amount: BalanceOf<T> },
    ProxyAdded { user: T::AccountId, proxy: T::AccountId, permission: ProxyPermission },
    ProxyRemoved { user: T::AccountId, proxy: T::AccountId },
    ProxyExecuted { user: T::AccountId, proxy: T::AccountId, fee: BalanceOf<T> },
    BatchExecuted { user: T::AccountId, proxy: T::AccountId, count: u32, total_fee: BalanceOf<T> },
    ServiceFeeCollected { amount: BalanceOf<T> },
    /// 系统托管池资金增加 [增加金额, 新总额]
    SystemPoolIncreased { amount: BalanceOf<T>, new_total: BalanceOf<T> },
}
```

## 错误处理

### 主要错误类型
```rust
pub enum Error<T> {
    /// 余额不足
    InsufficientBalance,
    
    /// 代理不存在
    ProxyNotFound,
    
    /// 代理未启用
    ProxyDisabled,
    
    /// 权限不足
    InsufficientPermission,
    
    /// 超出费用限额
    ExceedsFeeLimit,
    
    /// 批量交易过多
    TooManyBatchCalls,
    
    /// 最小充值金额不足
    BelowMinimumDeposit,
    
    /// 代理已存在
    ProxyAlreadyExists,
    
    /// 数值溢出
    Overflow,
}
```

## 查询功能

### 余额查询
```rust
// 查询用户预付费余额
let balance = pallet_paymaster::PrepaidBalances::<Runtime>::get(&user_account);

// 查询用户总费用消耗
let total_consumed = pallet_paymaster::Pallet::<Runtime>::get_total_fee_consumed(&user_account);
```

### 代理配置查询
```rust
// 查询特定代理配置
let config = pallet_paymaster::ProxyConfigs::<Runtime>::get(&user_account, &proxy_account);

// 遍历用户所有代理
for (proxy, config) in pallet_paymaster::ProxyConfigs::<Runtime>::iter_prefix(&user_account) {
    // 处理代理配置
}
```

### 费用记录查询
```rust
// 查询用户费用使用历史
let records = pallet_paymaster::FeeRecords::<Runtime>::get(&user_account);

// 查询系统总服务费收入
let total_fees = pallet_paymaster::TotalServiceFees::<Runtime>::get();
```

## 安全考虑

### 1. 权限控制
- 严格的权限验证机制，防止代理超权限操作
- 费用限额控制，防止恶意消耗用户资金
- 代理启用/禁用机制，提供紧急停止功能

### 2. 资金安全
- 预付费资金存储在模块账户中，与个人账户隔离
- 完整的费用追踪记录，确保资金使用透明
- 用户可随时提取未使用的预付费余额

### 3. 防护机制
- 最小充值金额限制，防止小额攻击
- 批量交易数量限制，防止资源滥用
- 数值溢出检查，确保计算安全

## 使用场景

### 1. DeFi 应用
- **流动性挖矿**: 用户无需持有原生代币即可参与挖矿
- **交易聚合**: 批量执行多个DEX交易，降低总体费用
- **自动化策略**: 代理执行复杂的DeFi策略

### 2. 游戏应用
- **道具交易**: 玩家无需关心gas费用，专注游戏体验
- **批量操作**: 一次性处理多个游戏内交易
- **新手友好**: 降低新用户进入门槛

### 3. 企业应用
- **员工津贴**: 企业为员工预付交易费用
- **批量发薪**: 一次性向多个员工发放薪资
- **供应链**: 简化供应链中的多方交易流程

## 最佳实践

### 1. 代理设置
- 为不同用途设置不同的代理账户
- 合理设置费用限额，平衡安全性和便利性
- 定期审查和更新代理权限

### 2. 费用管理
- 根据使用频率合理充值预付费
- 监控费用使用情况，及时调整策略
- 利用批量交易功能降低总体成本

### 3. 安全建议
- 使用专用的代理账户，避免与其他功能混用
- 定期检查代理配置，及时移除不需要的代理
- 保持代理账户的私钥安全

## 集成指南

### 1. 添加依赖
在 `Cargo.toml` 中添加：
```toml
pallet-paymaster = { path = "../pallets/paymaster", default-features = false }
```

### 2. 运行时配置
在 `runtime/src/lib.rs` 中配置pallet参数和实现Config trait。

### 3. 创世配置
```rust
// 在创世配置中设置初始参数
GenesisConfig {
    paymaster: PaymasterConfig {
        // 初始配置参数
    },
}
```

### 4. 前端集成
```javascript
// 使用 Polkadot.js API 调用 paymaster 功能
const api = await ApiPromise.create({ provider: wsProvider });

// 充值预付费
const depositTx = api.tx.paymaster.depositPrepaid(amount);

// 添加代理
const addProxyTx = api.tx.paymaster.addProxy(proxyAccount, permission, feeLimit);

// 代理执行交易
const proxyTx = api.tx.paymaster.proxyExecute(userAccount, call, estimatedFee);
```

## 版本历史

### v0.1.0
- 初始版本发布
- 基础预付费功能
- 代理权限管理
- 单笔和批量交易支持
- 费用追踪系统

## 许可证

本项目采用 MIT-0 许可证。详见 [LICENSE](../../LICENSE) 文件。

## 贡献指南

欢迎提交 Issue 和 Pull Request 来改进这个 pallet。在提交代码前，请确保：

1. 代码通过所有测试
2. 遵循 Rust 编码规范
3. 添加适当的文档注释
4. 更新相关文档

## 支持

如有问题或需要技术支持，请通过以下方式联系：

- 提交 GitHub Issue
- 发送邮件至项目维护者
- 加入项目讨论群组


## 免签名代付机制（预授权 + 系统托管池 + 交易扩展）

- 设计目标：在特定场景下为用户自动代付 Gas/费用，无需第三方实时签名，同时确保资金安全与系统可控。
- 安全控制：
  - 预授权：为赞助方设置额度上限、每笔上限、过期高度、每块处理上限
  - 系统托管池：提前注资至 Pallet 托管账户，避免实时签名
  - 队列限速：PendingAutoPays 队列 + 每区块最多处理 MaxAutoPayPerBlock
  - 白名单（可选）：SponsorWhitelist 控制可用赞助方
- 事件：AutoPayAuthorized、AutoPayRevoked、SystemPoolFunded、AutoPayRequested、AutoPayProcessed、AutoPayFailed

### 外部调用（新增）
- authorize_sponsor(sponsor, amount, expire_at, per_tx_limit, per_block_limit)
- revoke_sponsor(sponsor)
- fund_system_pool(amount)
- request_auto_pay(sponsor, amount)
- deposit_for_user(beneficiary, amount)（保留常规路径）

### 运行流程
1. 管理员 authorize_sponsor 配置赞助方权限与额度
2. 赞助方/治理账户 fund_system_pool 注资
3. 用户 request_auto_pay 发起免签请求，进入队列
4. on_initialize 限速批处理，成功触发 AutoPayProcessed 事件

### 交易扩展（概述）
- 在 validate 阶段做不可变校验，在 prepare 阶段做上链准备
- 参考 Runtime 中的 AutoPayTxExtension 注册