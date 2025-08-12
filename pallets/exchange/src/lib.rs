#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::{dispatch::DispatchResult, pallet_prelude::*, traits::{tokens::fungible::Mutate, Currency, ExistenceRequirement::AllowDeath}};
use frame_system::pallet_prelude::*;
use sp_runtime::traits::Saturating;
use sp_std::vec::Vec;

use pallet_karma::{self as karma, KarmaBalance, RewardReason, KarmaProvider};

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// BUD代币接口（通常为Balances）
        type Currency: Currency<Self::AccountId>;
        /// paymaster pallet的账户ID
        #[pallet::constant]
        type PaymasterAccount: Get<Self::AccountId>;
        /// 黑洞账户ID（不可恢复的销毁地址）
        #[pallet::constant]
        type BlackholeAccount: Get<Self::AccountId>;
        /// 国库账户ID（如果未集成Treasury pallet，可用一个固定账户代表）
        #[pallet::constant]
        type TreasuryAccount: Get<Self::AccountId>;
        /// 兑换比例：1 BUD => X Karma（用定点数整数表示，如1000表示1:1000）
        #[pallet::constant]
        type ExchangeRate: Get<u128>;
        /// 分配比例：黑洞占比（基点，万分制），例如2000=20%
        #[pallet::constant]
        type BurnBps: Get<u32>;
        /// 分配比例：国库占比（基点），例如7000=70%
        #[pallet::constant]
        type TreasuryBps: Get<u32>;
        /// 分配比例：paymaster占比（基点），例如1000=10%
        #[pallet::constant]
        type PaymasterBps: Get<u32>;
        /// BPS的基数（例如10000）
        #[pallet::constant]
        type BpsDenominator: Get<u32>;
        /// 分配比例：share-mining 奖金池占比（基点）
        #[pallet::constant]
        type ShareMiningBps: Get<u32>;
        /// share-mining 奖金池账户ID
        #[pallet::constant]
        type ShareMiningAccount: Get<Self::AccountId>;
    }

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// 成功兑换Karma [用户, bud_in, karma_out, burn, treasury, paymaster, share_mining]
        Exchanged(T::AccountId, u128, KarmaBalance, u128, u128, u128, u128),
    }

    #[pallet::error]
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

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// 用户用BUD兑换Karma，并将BUD按比例分配到黑洞/国库/paymaster/share-mining
        /// - amount: 兑换的BUD数量
        /// 安全性：
        /// - 校验 BurnBps + TreasuryBps + PaymasterBps + ShareMiningBps == BpsDenominator
        /// - 先执行BUD转账，再发放Karma，保证会计一致性
        /// - 规避舍入损失：将 remainder 分配给国库
        /// - 失败则回滚
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(2,4))]
        pub fn exchange(origin: OriginFor<T>, amount: u128) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(amount > 0, Error::<T>::ZeroAmount);
            let bps_sum = T::BurnBps::get()
                .saturating_add(T::TreasuryBps::get())
                .saturating_add(T::PaymasterBps::get())
                .saturating_add(T::ShareMiningBps::get());
            ensure!(bps_sum == T::BpsDenominator::get(), Error::<T>::InvalidBps);

            // 计算分配
            let denom = T::BpsDenominator::get() as u128;
            let burn = amount.saturating_mul(T::BurnBps::get() as u128) / denom;
            let treasury = amount.saturating_mul(T::TreasuryBps::get() as u128) / denom;
            let paymaster = amount.saturating_mul(T::PaymasterBps::get() as u128) / denom;
            let share_mining = amount.saturating_mul(T::ShareMiningBps::get() as u128) / denom;
            // 规避舍入丢失：将剩余分配给国库
            let allocated = burn
                .saturating_add(treasury)
                .saturating_add(paymaster)
                .saturating_add(share_mining);
            let remainder = amount.saturating_sub(allocated);
            let treasury_total = treasury.saturating_add(remainder);

            // 转账：who -> 黑洞/国库/paymaster/share-mining
            let bh = T::BlackholeAccount::get();
            let tr = T::TreasuryAccount::get();
            let pm = T::PaymasterAccount::get();
            let sm = T::ShareMiningAccount::get();

            // 使用Balances::transfer(AllowDeath)
            T::Currency::transfer(&who, &bh, burn.into(), AllowDeath).map_err(|_| Error::<T>::TransferFailed)?;
            T::Currency::transfer(&who, &tr, treasury_total.into(), AllowDeath).map_err(|_| Error::<T>::TransferFailed)?;
            T::Currency::transfer(&who, &pm, paymaster.into(), AllowDeath).map_err(|_| Error::<T>::TransferFailed)?;
            T::Currency::transfer(&who, &sm, share_mining.into(), AllowDeath).map_err(|_| Error::<T>::TransferFailed)?;

            // 计算Karma并发放
            let karma_out: KarmaBalance = amount.saturating_mul(T::ExchangeRate::get()).into();
            karma::Pallet::<T>::reward_karma(&who, karma_out, RewardReason::ManualAdjust).map_err(|_| Error::<T>::KarmaMintFailed)?;

            Self::deposit_event(Event::Exchanged(who, amount, karma_out, burn, treasury_total, paymaster, share_mining));
            Ok(())
        }
    }
}