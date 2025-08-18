### pallet-authorizer 概览

**目标**：提供一个“可复用的全局授权/投票中心”，由主网代币 BUD 持币人治理，统一维护各业务命名空间（例如以 `PalletId` 派生的 8 字节标识）下的账户白名单，供任意 Pallet 查询授权，避免每个 Pallet 各自重复造轮子。

### 功能特性
- **命名空间授权**：以 8 字节 `Namespace`（推荐用业务 `PalletId` 的字节）为维度，维护 `(Namespace, AccountId)` 白名单。
- **BUD 持币投票**：任何人可提交提案，BUD 持币人按持币余额加权投票（最小骨架版）。
- **简单多数通过**：投票期结束后，赞成权重大于反对即通过，执行“增加/移除授权”。
- **可查询**：提供 `is_authorized(ns, who)` 只读查询接口，供各业务 Pallet 复用。

### 存储设计
- `Authorized<(Namespace, AccountId) => ()>`：授权白名单。
- `Proposals<u64 => Proposal>`：提案详情，包含 `ns`、`op`（增/删）、`target`、`proposer`、`deposit`、`end`、`aye`、`nay`、`executed`。
- `VotesOf<(ProposalId, AccountId) => VoteChoice>`：防重复投票的简单记录。
- `NextProposalId<u64>`：提案自增序号。

### 配置参数（Config）
- **Currency**：治理代币（BUD），用于权重与押注（reserve）。
- **MinDeposit**：提交提案所需最小押注（防 spam）。
- **VotingPeriod**：投票期（区块数）。
- **AdminOrigin**：可用于紧急操作或参数管理（本最小版未开放管理 extrinsic，仅作占位）。

### Extrinsics 接口
- `submit_proposal(origin, ns_bytes: [u8; 8], op_code: u8, target: AccountId)`
  - 提交授权变更提案，并保留 `MinDeposit` 押注。
  - `op_code`: 1=Add（增加授权），0=Remove（移除授权）。
  - 投票截止高度为 `current_block + VotingPeriod`。

- `vote(origin, id: u64, choice_code: u8)`
  - 对 `id` 号提案投票，按 `Currency::free_balance(origin)` 计权。
  - `choice_code`: 1=Aye，0=Nay。

- `execute(origin, id: u64)`
  - 到期后任何人可触发执行；若赞成权重大于反对，则对 `(ns, target)` 增/删授权。

### 事件（Events）
- `ProposalSubmitted { id, ns: [u8;8], op: u8, target, end }`
- `Voted { id, who, choice: u8, weight }`
- `Executed { id, success }`

### 错误（Errors）
- `ProposalNotFound`、`AlreadyExecuted`、`VotingClosed`、`AlreadyVoted`、`DepositTooLow`
- `InvalidOp`、`InvalidChoice`

### 查询接口（供 Pallet 复用）
- `pallet_authorizer::Pallet::<T>::is_authorized(ns: Namespace, who: &AccountId) -> bool`

### 命名空间建议
- 推荐直接使用业务 Pallet 的 `PalletId` 来确定命名空间：
```rust
use frame_support::PalletId;
const MY_PALLET_ID: PalletId = PalletId(*b"budd/sin");
let ns_bytes: [u8; 8] = *MY_PALLET_ID.as_ref();
```

### 业务接入示例（以 karma 为例）
1) 治理流程：
   - 提交提案：将某业务模块账户加入 karma 的授权白名单命名空间（例如 `b"karma___"`）。
   - BUD 持币人投票，投票期结束后执行提案，完成白名单变更。

2) 业务 Pallet 内部调用：
```rust
use frame_support::PalletId;
use sp_runtime::traits::AccountIdConversion;
use pallet_authorizer::pallet::Namespace;

const KARMA_NS: [u8; 8] = *b"karma___"; // Karma 的命名空间
const SIGNIN_PALLET: PalletId = PalletId(*b"budd/sin");

fn try_gain_karma<T: pallet_karma::pallet::Config + pallet_authorizer::pallet::Config>(
    user: &T::AccountId,
    amount: <pallet_karma::Pallet<T> as pallet_karma::pallet::KarmaCurrency<T::AccountId>>::Balance,
) -> DispatchResult {
    // 1. 由 PalletId 派生模块账户
    let caller: T::AccountId = SIGNIN_PALLET.into_account_truncating();
    // 2. 查询授权中心
    ensure!(pallet_authorizer::Pallet::<T>::is_authorized(Namespace(KARMA_NS), &caller), 
        pallet_karma::pallet::Error::<T>::NotAuthorized);
    // 3. 调用 karma 内部接口
    pallet_karma::Pallet::<T>::gain(&caller, user, amount, b"signin".to_vec())
}
```

### 运行时集成
- 在 `runtime/Cargo.toml` 中加入依赖：
  - `pallet-authorizer = { path = "../pallets/authorizer", default-features = false }`
- 在 `runtime/src/lib.rs` 注册 Pallet：
```rust
#[runtime::pallet_index(9)]
pub type Authorizer = pallet_authorizer;
```
- 在 `runtime/src/configs/mod.rs` 配置：
```rust
parameter_types! {
    pub const AuthorizerMinDeposit: Balance = EXISTENTIAL_DEPOSIT;
    pub const AuthorizerVotingPeriod: BlockNumber = DAYS; // 约一天
}

impl pallet_authorizer::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances; // BUD
    type MinDeposit = AuthorizerMinDeposit;
    type VotingPeriod = AuthorizerVotingPeriod;
    type AdminOrigin = frame_system::EnsureRoot<AccountId>;
}
```

### 治理流程（最小骨架版）
1) 发起者提交提案并保留押注 `MinDeposit`（暂不实现自动赎回）。
2) 持币人按当前余额投票（未实现快照/conviction）。
3) 投票期结束后，任何人可执行：赞成权重大于反对则通过，落地白名单变更。

### 安全与边界
- 仅作为“授权中心”，不直接调用业务 Pallet；业务通过查询接口自行做授权判定。
- 当前版未实现押注赎回、法定人数、最小参与度、conviction 等高级治理功能，适合快速落地与 PoC。

### 未来扩展
- 接入 `pallet-conviction-voting` / `pallet-referenda` 做计票，authorizer 仅落地白名单。
- 支持“操作级授权”：`(Namespace, Operation, AccountId)`。
- 提案存续期、最小押注、赎回与惩罚策略参数化并可由治理修改。
- 事件/历史索引、批量授权/撤销、白名单导出/导入。

### 约束与注意
- `Namespace` 为 8 字节固定长度，推荐直接使用 `PalletId` 字节。
- 计票采用 `Currency::free_balance` 的简单快照，可能受投票期间转账影响（可通过快照或锁仓权重改进）。


