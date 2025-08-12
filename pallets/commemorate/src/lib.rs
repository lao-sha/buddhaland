#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*, traits::Get};
use frame_system::pallet_prelude::*;
use sp_std::vec::Vec;
use codec::{Encode, Decode};
use scale_info::TypeInfo;

use pallet_karma::{KarmaProvider, KarmaBalance, MeritAction};

/// IPFS 文件信息
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct IpfsFile {
    /// IPFS 文件哈希
    pub hash: Vec<u8>,
    /// 文件类型 (image, audio, video, document)
    pub file_type: Vec<u8>,
    /// 文件大小（字节）
    pub size: u64,
    /// 文件名称
    pub name: Vec<u8>,
}

/// 祭念馆信息
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct MemorialInfo<AccountId> {
    /// 创建者
    pub creator: AccountId,
    /// 逝者姓名
    pub name: Vec<u8>,
    /// 生日
    pub birth_date: Option<Vec<u8>>,
    /// 逝世日期
    pub death_date: Option<Vec<u8>>,
    /// 纪念描述
    pub description: Vec<u8>,
    /// 创建时间戳
    pub created_at: u64,
    /// 总访问次数
    pub total_visits: u32,
    /// 总祭奠次数
    pub total_ceremonies: u32,
    /// 是否公开访问
    pub is_public: bool,
    /// 头像照片 IPFS 哈希
    pub avatar_ipfs: Option<Vec<u8>>,
    /// 纪念相册 IPFS 文件列表
    pub gallery_files: Vec<IpfsFile>,
}

/// 祭奠活动类型
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum CeremonyType {
    /// 传统祭奠活动
    Traditional(TraditionalCeremony),
    /// 现代纪念活动
    Modern(ModernCeremony),
}

/// 传统祭奠活动
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum TraditionalCeremony {
    /// 献花
    OfferFlowers,
    /// 点烛
    LightCandles,
    /// 烧香
    BurnIncense,
    /// 供奉食物
    OfferFood,
    /// 烧纸钱
    BurnPaperMoney,
    /// 磕头拜祭
    Kowtow,
}

/// 现代纪念活动
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum ModernCeremony {
    /// 献花
    OfferFlowers,
    /// 点祈福灯
    LightPrayerLamps,
    /// 写留言
    WriteMessage,
    /// 播放音乐
    PlayMusic,
    /// 分享回忆
    ShareMemories,
    /// 在线追思
    OnlineMemorial,
}

/// 祭奠记录
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct CeremonyRecord<AccountId> {
    /// 祭奠者
    pub participant: AccountId,
    /// 祭奠类型
    pub ceremony_type: CeremonyType,
    /// 消耗的 Karma
    pub karma_cost: KarmaBalance,
    /// 祭奠留言
    pub message: Vec<u8>,
    /// 祭奠时间戳
    pub timestamp: u64,
    /// 附件文件（图片、音频、视频等）IPFS 列表
    pub attached_files: Vec<IpfsFile>,
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_timestamp::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// 逝者姓名最大长度
        #[pallet::constant]
        type MaxNameLength: Get<u32>;
        
        /// 描述信息最大长度
        #[pallet::constant]
        type MaxDescriptionLength: Get<u32>;
        
        /// 留言最大长度
        #[pallet::constant]
        type MaxMessageLength: Get<u32>;
        
        /// 每个账户最多创建的祭念馆数量
        #[pallet::constant]
        type MaxMemorialsPerAccount: Get<u32>;
        
        /// 创建祭念馆消耗的 Karma
        #[pallet::constant]
        type CreateMemorialCost: Get<KarmaBalance>;
        
        /// 传统祭奠基础费用
        #[pallet::constant]
        type BaseTraditionalCost: Get<KarmaBalance>;
        /// 现代纪念基础费用
        #[pallet::constant]
        type BaseModernCost: Get<KarmaBalance>;

        /// IPFS 哈希最大长度
        #[pallet::constant]
        type MaxIpfsHashLength: Get<u32>;
        
        /// 文件名最大长度
        #[pallet::constant]
        type MaxFileNameLength: Get<u32>;
        
        /// 每个祭念馆最大文件数量
        #[pallet::constant]
        type MaxFilesPerMemorial: Get<u32>;
        
        /// 每次祭奠最大附件数量
        #[pallet::constant]
        type MaxFilesPerCeremony: Get<u32>;
    }

    /// 存储：祭念馆信息
    /// 映射：祭念馆ID -> 祭念馆信息
    #[pallet::storage]
    #[pallet::getter(fn memorials)]
    pub type Memorials<T: Config> = StorageMap<_, Blake2_128Concat, u32, MemorialInfo<T::AccountId>>;

    /// 存储：下一个祭念馆ID
    #[pallet::storage]
    #[pallet::getter(fn next_memorial_id)]
    pub type NextMemorialId<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// 存储：账户拥有的祭念馆
    /// 映射：账户ID -> 祭念馆ID列表
    #[pallet::storage]
    #[pallet::getter(fn account_memorials)]
    pub type AccountMemorials<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, BoundedVec<u32, T::MaxMemorialsPerAccount>, ValueQuery>;

    /// 存储：祭奠记录
    /// 映射：(祭念馆ID, 记录索引) -> 祭奠记录
    #[pallet::storage]
    #[pallet::getter(fn ceremony_records)]
    pub type CeremonyRecords<T: Config> = StorageDoubleMap<_, Blake2_128Concat, u32, Blake2_128Concat, u32, CeremonyRecord<T::AccountId>>;

    /// 存储：祭念馆的祭奠记录数量
    /// 映射：祭念馆ID -> 记录数量
    #[pallet::storage]
    #[pallet::getter(fn memorial_record_count)]
    pub type MemorialRecordCount<T: Config> = StorageMap<_, Blake2_128Concat, u32, u32, ValueQuery>;

    /// 事件：祭奠纪念相关事件
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// 祭念馆创建成功 [创建者, 祭念馆ID, 逝者姓名]
        MemorialCreated(T::AccountId, u32, Vec<u8>),
        /// 祭念馆信息更新 [祭念馆ID, 更新者]
        MemorialUpdated(u32, T::AccountId),
        /// 祭奠活动完成 [祭念馆ID, 祭奠者, 祭奠类型, 消耗Karma]
        CeremonyPerformed(u32, T::AccountId, CeremonyType, KarmaBalance),
        /// 祭念馆访问 [祭念馆ID, 访问者]
        MemorialVisited(u32, T::AccountId),
        /// IPFS 文件上传成功 [祭念馆ID, 文件哈希, 文件类型]
        IpfsFileUploaded(u32, Vec<u8>, Vec<u8>),
        /// 祭奠媒体文件添加 [祭念馆ID, 祭奠者, 文件数量]
        CeremonyMediaAdded(u32, T::AccountId, u32),
    }

    #[pallet::error]
    pub enum Error<T> {
        MemorialNotFound,          // 祭念馆不存在
        NameTooLong,              // 名称过长
        DescriptionTooLong,       // 描述过长
        MessageTooLong,           // 留言过长
        TooManyMemorials,         // 达到最大祭念馆数量限制
        NoPermission,             // 没有权限操作
        AccessDenied,             // 祭念馆不公开且无权限访问
        InvalidCeremonyType,      // 无效的祭奠类型
        IpfsHashTooLong,          // IPFS 哈希过长
        FileNameTooLong,          // 文件名过长
        TooManyFiles,             // 文件数量超限
        InvalidFileType,          // 无效的文件类型
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// 创建祭念馆
        /// 
        /// 为逝者创建专属的链上陵墓，支持设置基本信息和头像
        /// 
        /// # 参数
        /// - `name`: 逝者姓名
        /// - `birth_date`: 生日（可选）
        /// - `death_date`: 逝世日期（可选）
        /// - `description`: 纪念描述
        /// - `is_public`: 是否公开访问
        /// - `avatar_ipfs`: 头像照片的 IPFS 哈希（可选）
        #[pallet::weight(10_000)]
        pub fn create_memorial(
            origin: OriginFor<T>,
            name: Vec<u8>,
            birth_date: Option<Vec<u8>>,
            death_date: Option<Vec<u8>>,
            description: Vec<u8>,
            is_public: bool,
            avatar_ipfs: Option<Vec<u8>>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            // 验证输入参数
            ensure!(name.len() <= T::MaxNameLength::get() as usize, Error::<T>::NameTooLong);
            ensure!(description.len() <= T::MaxDescriptionLength::get() as usize, Error::<T>::DescriptionTooLong);
            
            if let Some(ref hash) = avatar_ipfs {
                ensure!(hash.len() <= T::MaxIpfsHashLength::get() as usize, Error::<T>::IpfsHashTooLong);
            }

            // 检查用户创建的祭念馆数量限制
            let mut account_memorials = AccountMemorials::<T>::get(&who);
            ensure!(
                account_memorials.len() < T::MaxMemorialsPerAccount::get() as usize,
                Error::<T>::TooManyMemorials
            );

            // 消耗 Karma
            let cost = T::CreateMemorialCost::get();
            pallet_karma::Pallet::<T>::consume_karma(&who, cost, MeritAction::SocialInteraction, b"Create Memorial".to_vec())
                .map_err(|_| Error::<T>::NoPermission)?;

            // 创建祭念馆
            let memorial_id = NextMemorialId::<T>::get();
            let timestamp = pallet_timestamp::Pallet::<T>::get();

            let memorial_info = MemorialInfo {
                creator: who.clone(),
                name: name.clone(),
                birth_date,
                death_date,
                description,
                created_at: timestamp,
                total_visits: 0,
                total_ceremonies: 0,
                is_public,
                avatar_ipfs,
                gallery_files: Vec::new(),
            };

            // 存储祭念馆信息
            Memorials::<T>::insert(&memorial_id, &memorial_info);
            NextMemorialId::<T>::set(memorial_id + 1);

            // 更新账户的祭念馆列表
            account_memorials.push(memorial_id);
            AccountMemorials::<T>::insert(&who, account_memorials);

            // 发出事件
            Self::deposit_event(Event::MemorialCreated(who, memorial_id, name));

            Ok(().into())
        }

        /// 为祭念馆添加媒体文件
        /// 
        /// 添加图片、音频、视频等媒体文件到祭念馆相册
        /// 
        /// # 参数
        /// - `memorial_id`: 祭念馆ID
        /// - `files`: 要添加的 IPFS 文件列表
        #[pallet::weight(10_000)]
        pub fn add_memorial_media(
            origin: OriginFor<T>,
            memorial_id: u32,
            files: Vec<IpfsFile>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            // 获取祭念馆信息
            let mut memorial = Memorials::<T>::get(&memorial_id)
                .ok_or(Error::<T>::MemorialNotFound)?;

            // 检查权限（只有创建者可以添加媒体）
            ensure!(memorial.creator == who, Error::<T>::NoPermission);

            // 验证文件参数
            for file in &files {
                ensure!(file.hash.len() <= T::MaxIpfsHashLength::get() as usize, Error::<T>::IpfsHashTooLong);
                ensure!(file.name.len() <= T::MaxFileNameLength::get() as usize, Error::<T>::FileNameTooLong);
                ensure!(file.file_type.len() <= 20, Error::<T>::InvalidFileType);
            }

            // 检查文件数量限制
            let total_files = memorial.gallery_files.len() + files.len();
            ensure!(
                total_files <= T::MaxFilesPerMemorial::get() as usize,
                Error::<T>::TooManyFiles
            );

            // 添加文件到祭念馆
            memorial.gallery_files.extend(files.clone());
            Memorials::<T>::insert(&memorial_id, &memorial);

            // 发出事件
            for file in files {
                Self::deposit_event(Event::IpfsFileUploaded(
                    memorial_id,
                    file.hash,
                    file.file_type,
                ));
            }

            Ok(().into())
        }

        /// 进行传统祭奠活动（支持媒体附件）
        /// 
        /// 在链上陵墓中进行传统祭奠仪式，可附加图片、音频、视频等媒体文件
        /// 
        /// # 参数
        /// - `memorial_id`: 祭念馆ID
        /// - `ceremony`: 传统祭奠类型
        /// - `message`: 祭奠留言
        /// - `karma_amount`: 可选的自定义Karma消费量
        /// - `attached_files`: 附加的媒体文件（图片、音频、视频等）
        #[pallet::weight(10_000)]
        pub fn perform_traditional_ceremony(
            origin: OriginFor<T>,
            memorial_id: u32,
            ceremony: TraditionalCeremony,
            message: Vec<u8>,
            karma_amount: Option<KarmaBalance>,
            attached_files: Vec<IpfsFile>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            // 验证留言长度
            ensure!(message.len() <= T::MaxMessageLength::get() as usize, Error::<T>::MessageTooLong);

            // 验证附件文件
            ensure!(
                attached_files.len() <= T::MaxFilesPerCeremony::get() as usize,
                Error::<T>::TooManyFiles
            );

            for file in &attached_files {
                ensure!(file.hash.len() <= T::MaxIpfsHashLength::get() as usize, Error::<T>::IpfsHashTooLong);
                ensure!(file.name.len() <= T::MaxFileNameLength::get() as usize, Error::<T>::FileNameTooLong);
            }

            // 获取祭念馆并检查访问权限
            let mut memorial = Memorials::<T>::get(&memorial_id)
                .ok_or(Error::<T>::MemorialNotFound)?;

            if !memorial.is_public && memorial.creator != who {
                return Err(Error::<T>::AccessDenied.into());
            }

            // 计算 Karma 消费
            let karma_cost = karma_amount.unwrap_or(T::BaseTraditionalCost::get());

            // 消耗 Karma
            let action_desc = format!("Traditional Ceremony: {:?}", ceremony);
            pallet_karma::Pallet::<T>::consume_karma(&who, karma_cost, MeritAction::SocialInteraction, action_desc.into_bytes())
                .map_err(|_| Error::<T>::NoPermission)?;

            // 记录祭奠活动
            let timestamp = pallet_timestamp::Pallet::<T>::get();
            let record_index = MemorialRecordCount::<T>::get(&memorial_id);
            
            let ceremony_record = CeremonyRecord {
                participant: who.clone(),
                ceremony_type: CeremonyType::Traditional(ceremony.clone()),
                karma_cost,
                message,
                timestamp,
                attached_files: attached_files.clone(),
            };

            CeremonyRecords::<T>::insert(&memorial_id, &record_index, &ceremony_record);
            MemorialRecordCount::<T>::insert(&memorial_id, record_index + 1);

            // 更新祭念馆统计
            memorial.total_ceremonies += 1;
            Memorials::<T>::insert(&memorial_id, &memorial);

            // 发出事件
            Self::deposit_event(Event::CeremonyPerformed(
                memorial_id,
                who.clone(),
                CeremonyType::Traditional(ceremony),
                karma_cost,
            ));

            // 如果有附件文件，发出媒体添加事件
            if !attached_files.is_empty() {
                Self::deposit_event(Event::CeremonyMediaAdded(
                    memorial_id,
                    who,
                    attached_files.len() as u32,
                ));
            }

            Ok(().into())
        }

        /// 进行现代纪念活动（支持媒体附件）
        /// 
        /// 在链上陵墓中进行现代纪念活动，可附加图片、音频、视频等媒体文件
        /// 
        /// # 参数
        /// - `memorial_id`: 祭念馆ID
        /// - `ceremony`: 现代纪念类型
        /// - `message`: 纪念留言
        /// - `karma_amount`: 可选的自定义Karma消费量
        /// - `attached_files`: 附加的媒体文件（图片、音频、视频等）
        #[pallet::weight(10_000)]
        pub fn perform_modern_ceremony(
            origin: OriginFor<T>,
            memorial_id: u32,
            ceremony: ModernCeremony,
            message: Vec<u8>,
            karma_amount: Option<KarmaBalance>,
            attached_files: Vec<IpfsFile>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            // 验证留言长度
            ensure!(message.len() <= T::MaxMessageLength::get() as usize, Error::<T>::MessageTooLong);

            // 验证附件文件
            ensure!(
                attached_files.len() <= T::MaxFilesPerCeremony::get() as usize,
                Error::<T>::TooManyFiles
            );

            for file in &attached_files {
                ensure!(file.hash.len() <= T::MaxIpfsHashLength::get() as usize, Error::<T>::IpfsHashTooLong);
                ensure!(file.name.len() <= T::MaxFileNameLength::get() as usize, Error::<T>::FileNameTooLong);
            }

            // 获取祭念馆并检查访问权限
            let mut memorial = Memorials::<T>::get(&memorial_id)
                .ok_or(Error::<T>::MemorialNotFound)?;

            if !memorial.is_public && memorial.creator != who {
                return Err(Error::<T>::AccessDenied.into());
            }

            // 计算 Karma 消费
            let karma_cost = karma_amount.unwrap_or(T::BaseModernCost::get());

            // 消耗 Karma
            let action_desc = format!("Modern Ceremony: {:?}", ceremony);
            pallet_karma::Pallet::<T>::consume_karma(&who, karma_cost, MeritAction::SocialInteraction, action_desc.into_bytes())
                .map_err(|_| Error::<T>::NoPermission)?;

            // 记录祭奠活动
            let timestamp = pallet_timestamp::Pallet::<T>::get();
            let record_index = MemorialRecordCount::<T>::get(&memorial_id);
            
            let ceremony_record = CeremonyRecord {
                participant: who.clone(),
                ceremony_type: CeremonyType::Modern(ceremony.clone()),
                karma_cost,
                message,
                timestamp,
                attached_files: attached_files.clone(),
            };

            CeremonyRecords::<T>::insert(&memorial_id, &record_index, &ceremony_record);
            MemorialRecordCount::<T>::insert(&memorial_id, record_index + 1);

            // 更新祭念馆统计
            memorial.total_ceremonies += 1;
            Memorials::<T>::insert(&memorial_id, &memorial);

            // 发出事件
            Self::deposit_event(Event::CeremonyPerformed(
                memorial_id,
                who.clone(),
                CeremonyType::Modern(ceremony),
                karma_cost,
            ));

            // 如果有附件文件，发出媒体添加事件
            if !attached_files.is_empty() {
                Self::deposit_event(Event::CeremonyMediaAdded(
                    memorial_id,
                    who,
                    attached_files.len() as u32,
                ));
            }

            Ok(().into())
        }

        // ... existing code for other functions ...
    }

    impl<T: Config> Pallet<T> {
        /// 获取账户创建的所有祭念馆
        pub fn get_account_memorials(account: &T::AccountId) -> Vec<u32> {
            Self::account_memorials(account).into_inner()
        }

        /// 获取祭念馆的所有祭奠记录
        pub fn get_memorial_records(memorial_id: u32) -> Vec<CeremonyRecord<T::AccountId>> {
            let count = Self::memorial_record_count(&memorial_id);
            (0..count)
                .filter_map(|i| Self::ceremony_records(&memorial_id, &i))
                .collect()
        }
    }
}