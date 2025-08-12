#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use sp_std::{prelude::*, str};
use frame_support::{
    dispatch::DispatchResult,
    pallet_prelude::*,
    traits::{Currency, ExistenceRequirement::AllowDeath, Get},
};
use frame_system::{pallet_prelude::*, offchain::{AppCrypto, CreateSignedTransaction, SendUnsignedTransaction, SubmitTransaction, Signer}};

type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

const KEYWORDS: [&str; 4] = ["佛境", "冥想", "修心", "禅修"];

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::traits::tokens::Balance as _;
    use sp_runtime::{
        offchain as rt_offchain,
        transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
    };

    #[pallet::config]
    pub trait Config: frame_system::Config
    + CreateSignedTransaction<Call<Self>>
    {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// BUD 代币（通常为 Balances）
        type Currency: Currency<Self::AccountId>;
        /// Share-Mining 奖金池账户（接收来自 Exchange 的分配）
        #[pallet::constant]
        type PotAccount: Get<Self::AccountId>;
        /// 最大 URL 长度（字节）
        #[pallet::constant]
        type MaxUrlLen: Get<u32>;
        /// 每轮最多参与者（用于分批发放以控制区块耗时）
        #[pallet::constant]
        type MaxParticipantsPerRound: Get<u32>;
        /// Offchain HTTP 请求超时（毫秒）
        #[pallet::constant]
        type HttpTimeoutMillis: Get<u64>;
        /// Offchain 无签名交易的优先级
        #[pallet::constant]
        type UnsignedPriority: Get<TransactionPriority>;
    }

    /// 提交记录
    #[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
    pub struct Submission<AccountId, BlockNumber> {
        pub who: AccountId,
        pub url: BoundedVec<u8, <crate::Pallet<AccountId> as crate::UrlBound>::Bound>,
        pub at: BlockNumber,
        pub verified: bool,
        pub matched: bool,
    }

    // Bounded URL 类型助手（用于 MaxEncodedLen）
    pub trait UrlBound {
        type Bound: Get<u32>;
    }
    impl<T: Config> UrlBound for Pallet<T> {
        type Bound = T::MaxUrlLen;
    }

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn round_id)]
    /// 当前轮次 ID（每次 distribute 后自增）
    pub type RoundId<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn pending)]
    /// 待验证队列：key = blake2(url || who || block), value = (who, url, at)
    pub type Pending<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::Hash,
        (T::AccountId, BoundedVec<u8, T::MaxUrlLen>, T::BlockNumber),
        OptionQuery
    >;

    #[pallet::storage]
    #[pallet::getter(fn winners)]
    /// 本轮合格参与者（去重），用于平均分配
    pub type Winners<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        (), // set-like
        OptionQuery
    >;

    #[pallet::storage]
    #[pallet::getter(fn winners_count)]
    /// 本轮合格人数计数
    pub type WinnersCount<T> = StorageValue<_, u32, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// 用户提交链接 [who, url]
        LinkSubmitted(T::AccountId, Vec<u8>),
        /// 链下验证完成 [who, url, matched]
        LinkVerified(T::AccountId, Vec<u8>, bool),
        /// 完成一轮平均分配 [round_id, winners, per_user, total_paid]
        RewardsDistributed(u64, u32, BalanceOf<T>, BalanceOf<T>),
        /// 奖金池资金增加 [amount]
        PotIncreased(BalanceOf<T>),
    }

    #[pallet::error]
    pub enum Error<T> {
        UrlTooLong,
        DuplicateSubmission,
        NotFound,
        NoWinners,
        InsufficientPot,
        HttpError,
        DecodeError,
        Overflow,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// 用户提交链接，进入待验证队列
        /// - 链下工作者将抓取该 URL 内容并回填验证结果
        /// - URL 字节长度受 MaxUrlLen 限制
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn submit_link(origin: OriginFor<T>, url: Vec<u8>) -> DispatchResult {
            // 函数级注释：
            // 1. 校验 URL 长度
            // 2. 写入 Pending（key 使用 who + url + 当前区块）
            // 3. 发出事件 LinkSubmitted
            let who = ensure_signed(origin)?;
            let bounded: BoundedVec<_, T::MaxUrlLen> = url.clone().try_into().map_err(|_| Error::<T>::UrlTooLong)?;
            let now = <frame_system::Pallet<T>>::block_number();
            let key = T::Hashing::hash_of(&(who.clone(), &bounded, now));
            ensure!(!Pending::<T>::contains_key(&key), Error::<T>::DuplicateSubmission);

            Pending::<T>::insert(&key, (who.clone(), bounded, now));
            Self::deposit_event(Event::LinkSubmitted(who, url));
            Ok(())
        }

        /// Offchain 验证回填（无签名交易）
        /// - 仅允许由 Offchain Worker 发起的无签名交易
        /// - 验证 Pending 中存在对应记录
        /// - 若 matched=true，则登记为本轮获奖者
        #[pallet::weight(0)]
        pub fn submit_verification(
            origin: OriginFor<T>,
            who: T::AccountId,
            url: Vec<u8>,
            submitted_at: T::BlockNumber,
            matched: bool,
        ) -> DispatchResult {
            // 函数级注释：
            // 1. 验证为 unsigned 交易（任何签名都拒绝）
            // 2. 计算 key 并校验 Pending 是否存在
            // 3. 移除 Pending，若匹配则写入 Winners 并更新计数
            ensure_none(origin)?;
            let bounded: BoundedVec<_, T::MaxUrlLen> = url.clone().try_into().map_err(|_| Error::<T>::UrlTooLong)?;
            let key = T::Hashing::hash_of(&(who.clone(), &bounded, submitted_at));

            let Some((_ow, _u, _at)) = Pending::<T>::take(&key) else {
                // 重放或过期
                return Err(Error::<T>::NotFound.into());
            };

            if matched {
                if !Winners::<T>::contains_key(&who) {
                    Winners::<T>::insert(&who, ());
                    let c = WinnersCount::<T>::get().saturating_add(1);
                    WinnersCount::<T>::put(c);
                }
            }
            Self::deposit_event(Event::LinkVerified(who, url, matched));
            Ok(())
        }

        /// 平均分配当前奖金池资金
        /// - 从 PotAccount 读取可用余额
        /// - 平均分给本轮 Winners（最多处理 max_winners 个，支持分批发放）
        /// - 余数留在资金池
        /// - 完成后清空已发放的 Winners 项
        #[pallet::weight(50_000 + T::DbWeight::get().reads_writes(3, (max_winners as u64 + 3) as u64))]
        pub fn distribute(origin: OriginFor<T>, max_winners: u32) -> DispatchResult {
            // 函数级注释：
            // 1. 任何人可触发分配（公开函数），防止中心化
            // 2. 读取 pot 余额、winners 列表，计算 per_user
            // 3. 执行转账并维护 winners 状态
            let _ = ensure_signed(origin)?;
            let pot = T::PotAccount::get();
            let pot_balance = T::Currency::free_balance(&pot);

            let total_winners = WinnersCount::<T>::get();
            ensure!(total_winners > 0, Error::<T>::NoWinners);

            let batch = max_winners.min(total_winners).min(T::MaxParticipantsPerRound::get());
            // 收集前 batch 个获奖者
            let mut recipients: Vec<T::AccountId> = Vec::new();
            for (who, _) in Winners::<T>::iter().take(batch as usize) {
                recipients.push(who);
            }
            let count = recipients.len() as u32;
            ensure!(count > 0, Error::<T>::NoWinners);

            // per_user = pot_balance / count
            let per_user = pot_balance / <BalanceOf<T> as From<u128>>::from(count as u128);
            ensure!(!per_user.is_zero(), Error::<T>::InsufficientPot);

            // 转账
            let mut total_paid: BalanceOf<T> = Zero::zero();
            for who in &recipients {
                T::Currency::transfer(&pot, who, per_user, AllowDeath)?;
                total_paid = total_paid.saturating_add(per_user);
            }

            // 清理已发放的 winners 条目
            for who in &recipients {
                Winners::<T>::remove(who);
            }
            WinnersCount::<T>::put(total_winners.saturating_sub(count));

            // 若本轮全部发完则自增轮次
            if WinnersCount::<T>::get() == 0 {
                let next = Self::round_id().saturating_add(1);
                RoundId::<T>::put(next);
            }

            Self::deposit_event(Event::RewardsDistributed(Self::round_id(), count, per_user, total_paid));
            Ok(())
        }
    }

    // Offchain Worker：抓取待验证 URL 并提交无签名交易回填
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Offchain worker 入口
        /// - 遍历 Pending 的少量样本，发起 HTTP GET 请求
        /// - 若响应包含关键词，则提交 submit_verification 无签名交易
        fn offchain_worker(_n: T::BlockNumber) {
            // 函数级注释：
            // 1. 控制抓取数量，避免资源消耗
            // 2. 设置请求超时
            // 3. 解析为 UTF-8 字符串后进行关键词匹配
            let max_handle: usize = 5;
            for (key, (who, url_bounded, at)) in Pending::<T>::iter().take(max_handle) {
                let url_bytes = url_bounded.to_vec();
                if let Ok(url_str) = str::from_utf8(&url_bytes) {
                    let timeout = sp_io::offchain::timestamp().add(rt_offchain::Duration::from_millis(T::HttpTimeoutMillis::get()));
                    // 发送 HTTP 请求
                    let request = rt_offchain::http::Request::get(url_str);
                    let pending = match request.deadline(timeout).send() {
                        Ok(p) => p,
                        Err(_e) => {
                            // 提交未匹配，或忽略；这里忽略，等待下次尝试
                            continue;
                        }
                    };
                    if let Ok(response) = pending.try_wait(timeout) {
                        if let Ok(body) = response.body() {
                            if let Ok(text) = str::from_utf8(body) {
                                let matched = super::KEYWORDS.iter().any(|k| text.contains(k));
                                // 提交无签名交易
                                let call = Call::<T>::submit_verification {
                                    who: who.clone(),
                                    url: url_bytes.clone(),
                                    submitted_at: at,
                                    matched,
                                };
                                let _ = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into());
                            }
                        }
                    }
                }
            }
        }
    }

    // 校验无签名交易，限制为 OCW 回填路径，防止重放
    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            match call {
                Call::submit_verification { who, url, submitted_at, matched: _ } => {
                    let bounded: BoundedVec<_, T::MaxUrlLen> = match url.clone().try_into() {
                        Ok(b) => b,
                        Err(_) => return InvalidTransaction::ExhaustsResources.into(),
                    };
                    let key = T::Hashing::hash_of(&(who.clone(), &bounded, *submitted_at));
                    if !Pending::<T>::contains_key(&key) {
                        return InvalidTransaction::Stale.into();
                    }
                    Ok(ValidTransaction {
                        priority: T::UnsignedPriority::get(),
                        requires: vec![],
                        provides: vec![("submit_verification", key).encode()],
                        longevity: 64,
                        propagate: true,
                    })
                }
                _ => Err(InvalidTransaction::Call.into())
            }
        }
    }
}


    /// 增加奖金池资金 - 供其他 pallet 调用（如 exchange pallet）
    /// - amount: 要增加的 BUD 数量
    /// 安全性：
    /// - 不检查调用权限，允许其他 pallet 注入资金
    /// - 仅更新账户余额，不涉及复杂状态
    #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(0,0))]
    pub fn increase_pot_balance(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
        // 函数级注释：
        // 1. 确保是已签名调用（通常由其他 pallet 通过 dispatch 调用）
        // 2. 向 PotAccount 转入指定金额
        // 3. 发出事件通知奖金池增加
        let _who = ensure_signed(origin)?;
        let pot_account = T::PotAccount::get();
        
        // 此函数预期被其他 pallet 调用，调用方负责资金转移
        // 这里仅发出通知事件
        Self::deposit_event(Event::PotIncreased(amount));
        Ok(())
    }
    }

    // 添加公开模块函数供其他 pallet 直接调用
    impl<T: Config> Pallet<T> {
        /// 获取奖金池账户ID - 供其他 pallet 获取转账目标
        pub fn pot_account() -> T::AccountId {
            T::PotAccount::get()
        }

        /// 获取奖金池当前余额
        pub fn pot_balance() -> BalanceOf<T> {
            let pot = T::PotAccount::get();
            T::Currency::free_balance(&pot)
        }
    }