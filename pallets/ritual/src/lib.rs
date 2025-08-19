#![cfg_attr(not(feature = "std"), no_std)]

// 函数级详细中文注释：
// 本 Pallet 提供“祭奠/法事”的通用原语抽象（ritual），支持上香/供灯/供花/捐赠/诵经等行为，
// 统一进行频率（冷却）、周期配额与燃尽时长的链上校验与统计累积，并通过事件对外通知。
// Pallet 的设计目标是与寺院/执行者/订单/陵园/逝者等业务 Pallet 保持低耦合，
// 通过公共 TargetId 软关联与 OnOfferingCommitted 回调进行解耦联动。

extern crate alloc;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use alloc::vec::Vec;
    use frame_support::{
        pallet_prelude::*,
        BoundedVec,
    };
    use frame_system::pallet_prelude::*;
    // 函数级中文注释：引入饱和运算与安全数值转换 Trait，供区块高度与 u32 之间的安全转换与加减使用。
    use frame_support::sp_runtime::traits::{Saturating, SaturatedConversion};
    // 函数级中文注释：引入手动解码所需 Trait，便于为自定义类型补充 DecodeWithMemTracking 实现。
    use codec::{Decode as _, DecodeLimit};

    // -------- 公共类型定义 --------

    /// 函数级中文注释：供奉类别，预置若干常见类型并保留自定义扩展位。
    #[derive(Encode, Decode, Clone, Copy, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum OfferingKind { Incense, Lamp, Flower, Donation, Chant, Custom(u16) }

    /// 函数级中文注释：供奉目标抽象。使用软关联以避免对具体业务 Pallet 的编译依赖。
    #[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum TargetId { Memorial(u64), Plot(u64), Deceased(u64), Temple(u64), Order(u64), Custom(u64) }

    /// 函数级中文注释：规则定义。含冷却、周期配额与燃尽。
    #[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, Default)]
    pub struct RitualRule {
        pub cooldown_blocks: u32,
        pub period_blocks: Option<u32>,
        pub max_per_period: Option<u32>,
        pub burnout_blocks: Option<u32>,
    }

    /// 函数级中文注释：Ritual 规格模板。仅描述规则与元数据，不含定价。
    #[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct RitualSpec<AccountId, BoundedCid> {
        pub id: u64,
        pub kind: OfferingKind,
        pub provider: Option<AccountId>,
        pub default_rule: RitualRule,
        pub meta_cid: Option<BoundedCid>,
        pub active: bool,
    }

    /// 函数级中文注释：一次供奉行为的最小链上记录。证据等敏感数据建议链下（CID/承诺）。
    #[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct RitualAction<AccountId, BlockNumber, BoundedMemo> {
        pub action_id: u64,
        pub spec_id: u64,
        pub target: TargetId,
        pub kind: OfferingKind,
        pub actor: AccountId,
        pub at: BlockNumber,
        pub memo: Option<BoundedMemo>,
    }

    /// 函数级中文注释：供奉提交后对外回调 Trait。默认 Noop，通过 runtime 配置为订单等模块提供桥接。
    pub trait OnOfferingCommitted<AccountId, BlockNumber, BoundedMemo> {
        fn on_offering_committed(_action: &RitualAction<AccountId, BlockNumber, BoundedMemo>) {}
    }

    impl<AccountId, BlockNumber, BoundedMemo> OnOfferingCommitted<AccountId, BlockNumber, BoundedMemo> for () {}

    /// 函数级中文注释：目标控制 Trait。用于限定某些 Target 是否允许供奉，以及判断目标所有者权限（用于目标级规则覆盖）。
    pub trait TargetControl<AccountId> {
        fn is_allowed_target(_target: &TargetId) -> bool { true }
        fn is_owner(_target: &TargetId, _who: &AccountId) -> bool { false }
    }
    impl<AccountId> TargetControl<AccountId> for () {}

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// 函数级中文注释：命名空间常量，用于与 Authorizer/Forwarder 组合进行代付白名单域隔离。
        #[pallet::constant]
        type RitualNsBytes: Get<[u8; 8]>;

        /// 函数级中文注释：上限常量配置，限制资源消耗与 DoS 面。
        #[pallet::constant] type MaxSpecs: Get<u32>;
        #[pallet::constant] type MaxSpecsPerKind: Get<u32>;
        #[pallet::constant] type MaxSpecsPerProvider: Get<u32>;
        #[pallet::constant] type MaxMemoLen: Get<u32>;
        #[pallet::constant] type MaxCidLen: Get<u32>;
        #[pallet::constant] type MaxBatchOffers: Get<u32>;

        /// 函数级中文注释：缺省规则兜底（当规格未配置相应字段时）。
        #[pallet::constant] type DefaultCooldownBlocks: Get<u32>;
        #[pallet::constant] type DefaultBurnoutBlocks: Get<u32>;

        /// 函数级中文注释：目标控制与跨模块回调（低耦合）。
        type TargetControl: TargetControl<Self::AccountId>;
        type OnOfferingCommitted: OnOfferingCommitted<Self::AccountId, BlockNumberFor<Self>, BoundedVec<u8, Self::MaxMemoLen>>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // -------- 存储项 --------

    type BoundedCidOf<T> = BoundedVec<u8, <T as Config>::MaxCidLen>;
    type BoundedMemoOf<T> = BoundedVec<u8, <T as Config>::MaxMemoLen>;

    #[pallet::storage]
    pub type Specs<T: Config> = StorageMap<_, Blake2_128Concat, u64, RitualSpec<T::AccountId, BoundedCidOf<T>>, OptionQuery>;

    #[pallet::storage]
    pub type SpecsByKind<T: Config> = StorageMap<_, Blake2_128Concat, OfferingKind, BoundedVec<u64, T::MaxSpecsPerKind>, ValueQuery>;

    #[pallet::storage]
    pub type SpecsByProvider<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, BoundedVec<u64, T::MaxSpecsPerProvider>, ValueQuery>;

    #[pallet::storage]
    pub type Actions<T: Config> = StorageMap<_, Blake2_128Concat, u64, RitualAction<T::AccountId, BlockNumberFor<T>, BoundedMemoOf<T>>, OptionQuery>;

    #[pallet::storage]
    pub type TallyByTargetKind<T: Config> = StorageMap<_, Blake2_128Concat, (TargetId, OfferingKind), u64, ValueQuery>;

    #[pallet::storage]
    pub type LastActionAt<T: Config> = StorageMap<_, Blake2_128Concat, (TargetId, OfferingKind), BlockNumberFor<T>, OptionQuery>;

    #[pallet::storage]
    pub type BurnoutUntil<T: Config> = StorageMap<_, Blake2_128Concat, (TargetId, OfferingKind), BlockNumberFor<T>, OptionQuery>;

    #[pallet::storage]
    pub type PeriodWindow<T: Config> = StorageMap<_, Blake2_128Concat, (TargetId, OfferingKind), (BlockNumberFor<T>, u32), OptionQuery>;

    #[pallet::storage]
    pub type TargetRuleOverride<T: Config> = StorageMap<_, Blake2_128Concat, (TargetId, OfferingKind), RitualRule, OptionQuery>;

    #[pallet::storage]
    pub type NextSpecId<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    pub type NextActionId<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// 函数级中文注释：注册规格事件（kind 使用离散码，避免 DecodeWithMemTracking 约束）。
        /// kind: 0=Incense,1=Lamp,2=Flower,3=Donation,4=Chant,255=Custom
        SpecRegistered { spec_id: u64, kind: u8, provider: Option<T::AccountId> },
        /// 函数级中文注释：规格更新事件。
        SpecUpdated { spec_id: u64 },
        /// 函数级中文注释：供奉提交事件（target 以压缩形式 (code,id) 表达；kind 使用离散码）。
        /// target_code: 0=Memorial,1=Plot,2=Deceased,3=Temple,4=Order,255=Custom
        OfferingCommitted { action_id: u64, spec_id: u64, target_code: u8, target_id: u64, kind: u8, actor: T::AccountId, at: BlockNumberFor<T> },
        /// 函数级中文注释：目标级规则覆盖事件（压缩表示）。
        RuleOverridden { target_code: u8, target_id: u64, kind: u8 },
    }

    #[pallet::error]
    pub enum Error<T> {
        NotAuthorized,
        SpecNotFound,
        SpecInactive,
        TargetNotAllowed,
        CooldownActive,
        BurnoutActive,
        PeriodQuotaExceeded,
        Overflow,
        InvalidKind,
        InvalidTarget,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// 函数级中文注释：注册 ritual 规格模板。权限应由外部 Authorizer 控制，这里仅示例最小校验。
        /// - 参数：kind、default_rule、meta_cid（链下说明）
        /// - 效果：分配自增 id，写入索引；active=true；触发 SpecRegistered
        #[pallet::weight(10_000)]
        pub fn register_spec(
            origin: OriginFor<T>,
            kind: u8,
            default_rule: (u32, Option<u32>, Option<u32>, Option<u32>),
            meta_cid: Option<BoundedCidOf<T>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let spec_id = NextSpecId::<T>::mutate(|n| { let id = *n; *n = n.saturating_add(1); id });
            let kind_enum = Self::kind_from_u8(kind)?;
            let default_rule = Self::rule_from_tuple(default_rule);
            let spec = RitualSpec::<T::AccountId, BoundedCidOf<T>> { id: spec_id, kind: kind_enum, provider: Some(who.clone()), default_rule, meta_cid, active: true };
            ensure!(Specs::<T>::get(spec_id).is_none(), Error::<T>::Overflow);
            Specs::<T>::insert(spec_id, spec);
            SpecsByKind::<T>::mutate(kind_enum, |v| { let _ = v.try_push(spec_id); });
            SpecsByProvider::<T>::mutate(&who, |v| { let _ = v.try_push(spec_id); });
            Self::deposit_event(Event::<T>::SpecRegistered { spec_id, kind, provider: Some(who) });
            Ok(())
        }

        /// 函数级中文注释：更新 ritual 规格（规则/上下架/元数据）。仅示例最小变更路径。
        #[pallet::weight(10_000)]
        pub fn update_spec(
            origin: OriginFor<T>,
            spec_id: u64,
            patch_rule: Option<(u32, Option<u32>, Option<u32>, Option<u32>)>,
            active: Option<bool>,
            meta_cid: Option<Option<BoundedCidOf<T>>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Specs::<T>::try_mutate(spec_id, |maybe| -> DispatchResult {
                let s = maybe.as_mut().ok_or(Error::<T>::SpecNotFound)?;
                // 简化示例：允许注册者或任何人更新（实际应鉴权）。
                let _ = who; // TODO: 接入 Authorizer 校验
                if let Some(r) = patch_rule { s.default_rule = Self::rule_from_tuple(r); }
                if let Some(a) = active { s.active = a; }
                if let Some(cid_opt) = meta_cid { s.meta_cid = cid_opt; }
                Ok(())
            })?;
            Self::deposit_event(Event::<T>::SpecUpdated { spec_id });
            Ok(())
        }

        /// 函数级中文注释：提交一次供奉。
        /// - 校验：规格存在&激活；目标允许；冷却/燃尽/周期配额满足
        /// - 更新：自增 action、写 Actions/Tally/时间窗；触发事件与回调
        #[pallet::weight(10_000)]
        pub fn offer_tribute(
            origin: OriginFor<T>,
            spec_id: u64,
            target: (u8, u64),
            memo: Option<BoundedMemoOf<T>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let now = <frame_system::Pallet<T>>::block_number();

            let spec = Specs::<T>::get(spec_id).ok_or(Error::<T>::SpecNotFound)?;
            ensure!(spec.active, Error::<T>::SpecInactive);
            let target = Self::target_from_tuple(target)?;
            ensure!(T::TargetControl::is_allowed_target(&target), Error::<T>::TargetNotAllowed);

            let kind = spec.kind;
            let rule = TargetRuleOverride::<T>::get((target.clone(), kind)).unwrap_or(RitualRule {
                cooldown_blocks: T::DefaultCooldownBlocks::get(),
                period_blocks: None,
                max_per_period: None,
                burnout_blocks: Some(T::DefaultBurnoutBlocks::get()),
            }).or_default_merge(spec.default_rule);

            // 冷却校验
            if let Some(last) = LastActionAt::<T>::get((target.clone(), kind)) {
                if rule.cooldown_blocks > 0 {
                    ensure!(
                        now.saturating_sub(last) >= rule.cooldown_blocks.saturated_into(),
                        Error::<T>::CooldownActive
                    );
                }
            }
            // 燃尽校验
            if let Some(until) = BurnoutUntil::<T>::get((target.clone(), kind)) {
                ensure!(now >= until, Error::<T>::BurnoutActive);
            }
            // 周期配额校验
            if let (Some(period), Some(maxn)) = (rule.period_blocks, rule.max_per_period) {
                if let Some((start, used)) = PeriodWindow::<T>::get((target.clone(), kind)) {
                    let passed: BlockNumberFor<T> = now.saturating_sub(start);
                    if passed < period.saturated_into() {
                        ensure!(used < maxn, Error::<T>::PeriodQuotaExceeded);
                    }
                }
            }

            // 写入与更新
            let action_id = NextActionId::<T>::mutate(|n| { let id = *n; *n = n.saturating_add(1); id });
            let action = RitualAction::<T::AccountId, BlockNumberFor<T>, BoundedMemoOf<T>> { action_id, spec_id, target: target.clone(), kind, actor: who.clone(), at: now, memo };
            Actions::<T>::insert(action_id, &action);
            TallyByTargetKind::<T>::mutate((target.clone(), kind), |c| *c = c.saturating_add(1));
            LastActionAt::<T>::insert((target.clone(), kind), now);
            if let Some(burn) = rule.burnout_blocks {
                if burn > 0 {
                    BurnoutUntil::<T>::insert(
                        (target.clone(), kind),
                        now.saturating_add(burn.saturated_into()),
                    );
                }
            }
            if let (Some(period), Some(maxn)) = (rule.period_blocks, rule.max_per_period) {
                PeriodWindow::<T>::mutate((target.clone(), kind), |w| {
                    match w {
                        Some((start, used)) => {
                            let passed: BlockNumberFor<T> = now.saturating_sub(*start);
                            if passed < period.saturated_into() {
                                let new_used = used.saturating_add(1);
                                *w = Some((*start, new_used.min(maxn)));
                            } else {
                                *w = Some((now, 1));
                            }
                        }
                        None => *w = Some((now, 1)),
                    }
                });
            }

            let (t_code, t_id) = Self::target_to_tuple(&target);
            let k_code = Self::kind_to_u8(kind);
            Self::deposit_event(Event::<T>::OfferingCommitted { action_id, spec_id, target_code: t_code, target_id: t_id, kind: k_code, actor: who.clone(), at: now });
            <T as Config>::OnOfferingCommitted::on_offering_committed(&action);
            Ok(())
        }

        /// 函数级中文注释：批量供奉。遍历 items 逐项执行与 `offer_tribute` 相同的校验与写入逻辑，
        /// 任一项失败则整体回滚。
        #[pallet::weight(10_000)]
        pub fn batch_offer_tribute(
            origin: OriginFor<T>,
            items: BoundedVec<(u64, (u8, u64), Option<BoundedMemoOf<T>>), T::MaxBatchOffers>,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;
            for (spec, target, memo) in items.into_inner() {
                // 直接调用内部逻辑以保持一致性
                // 注意：这里为了简化示例，重复读取与写入；正式实现建议抽取内部函数减少重复
                Self::offer_tribute(frame_system::RawOrigin::Signed(_who.clone()).into(), spec, target, memo)?;
            }
            Ok(())
        }

        /// 函数级中文注释：设置目标级规则覆盖。允许更严格的控制。
        #[pallet::weight(10_000)]
        pub fn set_target_rule_override(
            origin: OriginFor<T>,
            target: (u8, u64),
            kind: u8,
            rule: Option<(u32, Option<u32>, Option<u32>, Option<u32>)>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let target = Self::target_from_tuple(target)?;
            let kind_enum = Self::kind_from_u8(kind)?;
            ensure!(T::TargetControl::is_owner(&target, &who), Error::<T>::NotAuthorized);
            match rule { Some(r) => TargetRuleOverride::<T>::insert((target.clone(), kind_enum), Self::rule_from_tuple(r)), None => TargetRuleOverride::<T>::remove((target.clone(), kind_enum)), }
            let (t_code, t_id) = Self::target_to_tuple(&target);
            Self::deposit_event(Event::<T>::RuleOverridden { target_code: t_code, target_id: t_id, kind });
            Ok(())
        }
    }

    // -------- 辅助实现 --------
    impl RitualRule {
        /// 函数级中文注释：辅助合并，若自身字段为 None 则使用对方字段；用于生成最终规则。
        pub fn or_default_merge(self, other: RitualRule) -> RitualRule {
            RitualRule {
                cooldown_blocks: if self.cooldown_blocks > 0 { self.cooldown_blocks } else { other.cooldown_blocks },
                period_blocks: self.period_blocks.or(other.period_blocks),
                max_per_period: self.max_per_period.or(other.max_per_period),
                burnout_blocks: self.burnout_blocks.or(other.burnout_blocks),
            }
        }
    }

    // -------- 内部转换与压缩表示辅助 --------
    impl<T: Config> Pallet<T> {
        /// 函数级中文注释：从 u8 转 OfferingKind；不识别返回错误。
        fn kind_from_u8(code: u8) -> Result<OfferingKind, DispatchError> {
            Ok(match code {
                0 => OfferingKind::Incense,
                1 => OfferingKind::Lamp,
                2 => OfferingKind::Flower,
                3 => OfferingKind::Donation,
                4 => OfferingKind::Chant,
                255 => OfferingKind::Custom(0),
                _ => return Err(Error::<T>::InvalidKind.into()),
            })
        }

        /// 函数级中文注释：将 OfferingKind 转为离散码；自定义统一映射到 255。
        fn kind_to_u8(kind: OfferingKind) -> u8 {
            match kind {
                OfferingKind::Incense => 0,
                OfferingKind::Lamp => 1,
                OfferingKind::Flower => 2,
                OfferingKind::Donation => 3,
                OfferingKind::Chant => 4,
                OfferingKind::Custom(_) => 255,
            }
        }

        /// 函数级中文注释：从压缩表示 (code,id) 还原 TargetId。
        fn target_from_tuple(t: (u8, u64)) -> Result<TargetId, DispatchError> {
            Ok(match t.0 {
                0 => TargetId::Memorial(t.1),
                1 => TargetId::Plot(t.1),
                2 => TargetId::Deceased(t.1),
                3 => TargetId::Temple(t.1),
                4 => TargetId::Order(t.1),
                255 => TargetId::Custom(t.1),
                _ => return Err(Error::<T>::InvalidTarget.into()),
            })
        }

        /// 函数级中文注释：将 TargetId 压缩为 (code,id)。
        fn target_to_tuple(target: &TargetId) -> (u8, u64) {
            match target {
                TargetId::Memorial(x) => (0, *x),
                TargetId::Plot(x) => (1, *x),
                TargetId::Deceased(x) => (2, *x),
                TargetId::Temple(x) => (3, *x),
                TargetId::Order(x) => (4, *x),
                TargetId::Custom(x) => (255, *x),
            }
        }

        /// 函数级中文注释：从元组构造 RitualRule。
        fn rule_from_tuple(t: (u32, Option<u32>, Option<u32>, Option<u32>)) -> RitualRule {
            RitualRule { cooldown_blocks: t.0, period_blocks: t.1, max_per_period: t.2, burnout_blocks: t.3 }
        }
    }
}


