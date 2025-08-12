# Exchange Pallet

## 功能概述

Exchange Pallet专门处理BUD代币到Karma的单向兑换，支持将收到的BUD按照预设比例分配到不同的账户：
- 一部分进入黑洞销毁（通缩机制）
- 一部分进入国库（治理资金）
- 一部分进入Paymaster系统托管池用于支付用户Gas费
- 一部分进入Share-Mining奖金池用于支付share-mining奖励

## 核心特性

### 分配机制
- **黑洞销毁**: 不可恢复的BUD销毁，减少总供应量
- **国库**: 用于治理提案和链上开支
- **Paymaster系统托管池**: 免签名代付gas费资金池
- **Share-Mining 奖金池**: 用于支付 share-mining 激励

### 配置参数
- `ExchangeRate`: 兑换比例（1 BUD = X Karma）
- 分配比例（基点制）：
  - `BurnBps`: 黑洞销毁比例
  - `TreasuryBps`: 国库分配比例
  - `PaymasterBps`: Paymaster系统托管池分配比例
  - `ShareMiningBps`: Share-Mining 奖金池分配比例
- 账户配置：`BlackholeAccount`, `TreasuryAccount`
- `BpsDenominator`: 基点分母（通常10000），需满足 BurnBps + TreasuryBps + PaymasterBps + ShareMiningBps = BpsDenominator

## 存储结构

无链上存储，所有状态通过配置参数和事件维护。

## 事件

```rust
#[pallet::event]
pub enum Event<T: Config> {
    /// 成功兑换Karma [用户, bud_in, karma_out, burn, treasury, paymaster, share_mining]
    Exchanged(T::AccountId, u128, KarmaBalance, u128, u128, u128, u128),
}
```

### 事件说明

#### Exchanged
```rust
Exchanged(AccountId, u128, KarmaBalance, u128, u128, u128, u128)
```
- 参数：`[用户账户, BUD输入数量, Karma输出数量, 黑洞销毁数量, 国库分配数量, Paymaster数量, Share-Mining数量]`
- 触发时机：每次成功完成 BUD→Karma 兑换和分配后

## 外部调用接口

### exchange(amount: u128)

用户兑换接口，将指定数量的 BUD 兑换为 Karma，并按配置比例分配 BUD。

**执行流程**：
1. 校验输入参数（amount > 0）
2. 校验分配比例总和等于基点分母
3. 计算四个目标的分配金额（黑洞、国库、Paymaster系统托管池、Share-Mining 奖金池）
4. 依次转账到黑洞、国库、paymaster pallet账户、share-mining 奖金池账户
5. 计算并发放 Karma
6. 处理舍入损失（余数分配给国库）
7. 发出Exchanged事件

**错误处理**：
- `ZeroAmount`: 输入金额为0
- `InvalidBps`: 分配比例总和不等于基点分母
- `TransferFailed`: BUD转账失败
- `KarmaMintFailed`: Karma发放失败
- `PaymasterDepositFailed`: Paymaster系统托管池充值失败

## 与其他 Pallet 的集成

- **Karma Pallet**: 调用 `reward_karma()` 发放兑换得到的 Karma
- **Paymaster Pallet**: 调用 `increase_system_pool()` 将资金存入系统托管池而非简单转账
- **Share-Mining Pallet**: 直接转账到其奖金池账户（PotAccount）

## 运行时配置示例

```rust
impl pallet_exchange::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type BlackholeAccount = BlackholeAccountId;
    type TreasuryAccount = TreasuryAccountId;
    type ExchangeRate = ConstU128<1000>; // 1 BUD = 1000 Karma
    type BurnBps = ConstU32<2000>;       // 20%
    type TreasuryBps = ConstU32<7000>;   // 70%
    type PaymasterBps = ConstU32<800>;   // 8%
    type ShareMiningBps = ConstU32<200>; // 2%
    type BpsDenominator = ConstU32<10000>; // 基点分母
}
```

## 安全考虑

1. **算术安全**: 所有计算使用饱和运算避免溢出
2. **原子性**: 失败时自动回滚，保证一致性
3. **舍入处理**: 余数分配给国库，避免精度损失
4. **权限控制**: 仅允许已签名的用户发起兑换
5. **激励机制**: Share-mining奖金池提供持续的用户激励

## 总结

Exchange Pallet 通过将 BUD→Karma 兑换与多目标资金分配相结合，实现了代币经济的关键功能：通缩机制（黑洞）、治理资金（国库）、用户体验优化（Paymaster）和生态激励（Share-Mining），为佛境项目的可持续发展提供了坚实的经济基础。