#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*, traits::Get};
use frame_system::pallet_prelude::*;
use sp_std::vec::Vec;

use pallet_karma::{KarmaProvider, KarmaBalance, MeritAction};

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// 纪念祭祀 Pallet 配置项
    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// 各类纪念祭祀动作的默认消耗（可由 runtime 配置）
        #[pallet::constant]
        type DefaultIncenseCost: Get<KarmaBalance>;
        #[pallet::constant]
        type DefaultLampCost: Get<KarmaBalance>;
        #[pallet::constant]
        type DefaultFlowerCost: Get<KarmaBalance>;
        #[pallet::constant]
        type DefaultDonationCost: Get<KarmaBalance>;
    }

    /// 事件：纪念祭祀消费成功记录
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// 纪念祭祀事件：账户、动作、消耗数量、备注
        CommemorativeRitualPerformed(T::AccountId, MeritAction, KarmaBalance, Vec<u8>),
    }

    /// 错误类型
    #[pallet::error]
    pub enum Error<T> {
        /// 备注内容过长或无效
        InvalidNote,
        /// 无效的自定义动作编号
        InvalidCustomAction,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// 上香（Incense）- 纪念缅怀
        /// 
        /// # 参数
        /// * `origin` - 调用者来源
        /// * `note` - 纪念备注
        /// * `amount` - 可选的自定义 Karma 消费量
        /// 
        /// # 功能
        /// 燃香供佛，表达对逝者的纪念与缅怀
        #[pallet::weight(10_000)]
        pub fn incense(origin: OriginFor<T>, note: Vec<u8>, amount: Option<KarmaBalance>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let cost = amount.unwrap_or_else(|| T::DefaultIncenseCost::get());
            <pallet_karma::Pallet<T> as KarmaProvider<T::AccountId>>::consume_karma_for_merit(&who, cost, MeritAction::Incense)
                .map_err(|_| frame_support::dispatch::DispatchError::Other("ConsumeFailed"))?;
            Self::deposit_event(Event::CommemorativeRitualPerformed(who, MeritAction::Incense, cost, note));
            Ok(().into())
        }

        /// 点灯（LightLamp）- 照亮逝者归途
        /// 
        /// # 参数
        /// * `origin` - 调用者来源
        /// * `note` - 纪念备注
        /// * `amount` - 可选的自定义 Karma 消费量
        /// 
        /// # 功能
        /// 点燃明灯，为逝者照亮归途，祈求其在另一世界得到光明
        #[pallet::weight(10_000)]
        pub fn light_lamp(origin: OriginFor<T>, note: Vec<u8>, amount: Option<KarmaBalance>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let cost = amount.unwrap_or_else(|| T::DefaultLampCost::get());
            <pallet_karma::Pallet<T> as KarmaProvider<T::AccountId>>::consume_karma_for_merit(&who, cost, MeritAction::LightLamp)
                .map_err(|_| frame_support::dispatch::DispatchError::Other("ConsumeFailed"))?;
            Self::deposit_event(Event::CommemorativeRitualPerformed(who, MeritAction::LightLamp, cost, note));
            Ok(().into())
        }

        /// 供花（Flower）- 献花纪念
        /// 
        /// # 参数
        /// * `origin` - 调用者来源
        /// * `note` - 纪念备注
        /// * `amount` - 可选的自定义 Karma 消费量
        /// 
        /// # 功能
        /// 鲜花供养，以美好的花朵表达对逝者的怀念与敬意
        #[pallet::weight(10_000)]
        pub fn offer_flower(origin: OriginFor<T>, note: Vec<u8>, amount: Option<KarmaBalance>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let cost = amount.unwrap_or_else(|| T::DefaultFlowerCost::get());
            <pallet_karma::Pallet<T> as KarmaProvider<T::AccountId>>::consume_karma_for_merit(&who, cost, MeritAction::Flower)
                .map_err(|_| frame_support::dispatch::DispatchError::Other("ConsumeFailed"))?;
            Self::deposit_event(Event::CommemorativeRitualPerformed(who, MeritAction::Flower, cost, note));
            Ok(().into())
        }

        /// 布施/捐赠（Donation）- 功德回向
        /// 
        /// # 参数
        /// * `origin` - 调用者来源
        /// * `note` - 纪念备注
        /// * `amount` - 可选的自定义 Karma 消费量
        /// 
        /// # 功能
        /// 财物布施，将功德回向给逝者，愿其在来世得到庇佑
        #[pallet::weight(10_000)]
        pub fn donate(origin: OriginFor<T>, note: Vec<u8>, amount: Option<KarmaBalance>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let cost = amount.unwrap_or_else(|| T::DefaultDonationCost::get());
            <pallet_karma::Pallet<T> as KarmaProvider<T::AccountId>>::consume_karma_for_merit(&who, cost, MeritAction::Donation)
                .map_err(|_| frame_support::dispatch::DispatchError::Other("ConsumeFailed"))?;
            Self::deposit_event(Event::CommemorativeRitualPerformed(who, MeritAction::Donation, cost, note));
            Ok(().into())
        }

        /// 自定义纪念仪式（MeritAction::Other(code)）
        /// 
        /// # 参数
        /// * `origin` - 调用者来源
        /// * `code` - 自定义纪念仪式类型代码（必须 > 0）
        /// * `note` - 纪念备注
        /// * `amount` - 必须指定的 Karma 消费量
        /// 
        /// # 功能
        /// 支持用户定义的特殊纪念仪式，提供扩展性
        #[pallet::weight(10_000)]
        pub fn custom(origin: OriginFor<T>, code: u8, note: Vec<u8>, amount: KarmaBalance) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(code > 0, Error::<T>::InvalidCustomAction);
            <pallet_karma::Pallet<T> as KarmaProvider<T::AccountId>>::consume_karma_for_merit(&who, amount, MeritAction::Other(code))
                .map_err(|_| frame_support::dispatch::DispatchError::Other("ConsumeFailed"))?;
            Self::deposit_event(Event::CommemorativeRitualPerformed(who, MeritAction::Other(code), amount, note));
            Ok(().into())
        }
    }
}