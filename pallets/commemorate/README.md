# Commemorate Pallet

## 概述

`Commemorate Pallet` 是 BuddhaLand 佛境生态系统中的祭奠纪念模块，为用户提供创建和管理链上祭念馆的功能。每个用户可以为逝去的亲人、朋友或其他重要的人创建专属的数字纪念空间，并进行各种传统和现代的祭奠纪念活动。

该 pallet 支持：
- **自主创建**：每个账户可以无限创建祭念馆和链上陵墓
- **IPFS 存储**：祭奠图片、音频、视频、长文等大文件存储在 IPFS 上，区块链仅存储文件哈希
- **传统祭奠**：献花、点烛、烧香、供奉食物、烧纸钱、磕头拜祭等传统仪式
- **现代纪念**：献花、点祈福灯、写留言、播放音乐、分享回忆、在线追思等现代活动
- **多媒体支持**：支持图片、音频、视频等多种格式的纪念媒体
- **隐私控制**：支持公开和私密两种访问模式
- **永久保存**：所有祭奠记录都将永久保存在区块链上

## 术语对齐

- **祭念馆**：为逝者创建的数字纪念空间，类似于现实中的墓地或灵堂
- **链上陵墓**：祭念馆的别称，强调其区块链存储和永久性特征
- **祭奠活动**：在祭念馆中进行的各种纪念仪式
- **传统祭奠**：基于传统文化的祭祀仪式
- **现代纪念**：结合现代科技的纪念活动
- **IPFS 存储**：分布式文件存储系统，确保大文件的可靠存储
- **文件哈希**：IPFS 文件的唯一标识符，存储在区块链上
- **Karma消费**：进行祭奠活动需要消耗的功德值

## 依赖与集成

### 核心依赖
- **`pallet_karma`**：提供 Karma 系统支持，处理功德值的消费和奖励
- **`pallet_timestamp`**：提供时间戳服务，记录祭念馆创建和祭奠活动时间
- **`frame_system`**：Substrate 系统模块，提供基础账户和权限管理

### 外部依赖
- **IPFS 网络**：分布式文件存储，保证媒体文件的持久化和可访问性

### 集成设计
```rust
// Runtime 中的集成配置
impl pallet_commemorate::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type MaxNameLength = MaxNameLength;
    type MaxDescriptionLength = MaxDescriptionLength;
    type MaxMessageLength = MaxMessageLength;
    type MaxMemorialsPerAccount = MaxMemorialsPerAccount;
    type CreateMemorialCost = CreateMemorialCost;
    type BaseTraditionalCost = BaseTraditionalCost;
    type BaseModernCost = BaseModernCost;
    
    // IPFS 相关配置
    type MaxIpfsHashLength = MaxIpfsHashLength;
    type MaxFileNameLength = MaxFileNameLength;
    type MaxFilesPerMemorial = MaxFilesPerMemorial;
    type MaxFilesPerCeremony = MaxFilesPerCeremony;
}
```

## 类型定义与配置

### 核心类型

#### IpfsFile
```rust
pub struct IpfsFile {
    pub hash: Vec<u8>,      // IPFS 文件哈希
    pub file_type: Vec<u8>, // 文件类型 (image, audio, video, document)
    pub size: u64,          // 文件大小（字节）
    pub name: Vec<u8>,      // 文件名称
}
```

#### MemorialInfo
```rust
pub struct MemorialInfo<AccountId> {
    pub creator: AccountId,                // 创建者
    pub name: Vec<u8>,                    // 逝者姓名
    pub birth_date: Option<Vec<u8>>,      // 生日（可选）
    pub death_date: Option<Vec<u8>>,      // 逝世日期（可选）
    pub description: Vec<u8>,             // 纪念描述
    pub created_at: u64,                  // 创建时间戳
    pub total_visits: u32,                // 总访问次数
    pub total_ceremonies: u32,            // 总祭奠次数
    pub is_public: bool,                  // 是否公开访问
    pub avatar_ipfs: Option<Vec<u8>>,     // 头像照片 IPFS 哈希
    pub gallery_files: Vec<IpfsFile>,     // 纪念相册 IPFS 文件列表
}
```

#### CeremonyRecord
```rust
pub struct CeremonyRecord<AccountId> {
    pub participant: AccountId,           // 祭奠者
    pub ceremony_type: CeremonyType,     // 祭奠类型
    pub karma_cost: KarmaBalance,        // 消耗的 Karma
    pub message: Vec<u8>,                // 祭奠留言
    pub timestamp: u64,                  // 祭奠时间戳
    pub attached_files: Vec<IpfsFile>,   // 附件文件（图片、音频、视频等）
}
```

### 配置参数

| 参数 | 类型 | 说明 | 推荐值 |
|------|------|------|--------|
| `MaxNameLength` | `u32` | 逝者姓名最大长度 | 100 |
| `MaxDescriptionLength` | `u32` | 描述信息最大长度 | 1000 |
| `MaxMessageLength` | `u32` | 留言最大长度 | 500 |
| `MaxMemorialsPerAccount` | `u32` | 每账户最大祭念馆数 | 50 |
| `CreateMemorialCost` | `KarmaBalance` | 创建祭念馆费用 | 100 |
| `BaseTraditionalCost` | `KarmaBalance` | 传统祭奠基础费用 | 20 |
| `BaseModernCost` | `KarmaBalance` | 现代纪念基础费用 | 15 |
| `MaxIpfsHashLength` | `u32` | IPFS 哈希最大长度 | 64 |
| `MaxFileNameLength` | `u32` | 文件名最大长度 | 256 |
| `MaxFilesPerMemorial` | `u32` | 每个祭念馆最大文件数 | 100 |
| `MaxFilesPerCeremony` | `u32` | 每次祭奠最大附件数 | 10 |

## IPFS 集成说明

### 支持的文件类型
- **图片**：JPEG、PNG、GIF、WebP 等格式的祭奠照片和纪念图片
- **音频**：MP3、WAV、AAC 等格式的纪念音乐和语音留言
- **视频**：MP4、WebM、AVI 等格式的纪念视频和生活片段
- **文档**：PDF、TXT、DOC 等格式的长篇纪念文章和传记

### IPFS 存储流程
1. **客户端上传**：前端将媒体文件上传到 IPFS 网络
2. **获取哈希**：IPFS 返回文件的唯一哈希标识符
3. **链上存储**：将文件哈希、类型、大小等元数据存储到区块链
4. **关联祭念馆**：将文件与对应的祭念馆或祭奠记录关联

### 文件访问
- 通过 IPFS 哈希从分布式网络获取文件内容
- 支持多个 IPFS 网关确保文件可访问性
- 客户端可缓存常用文件提升访问速度

## 存储设计

### 主要存储映射

1. **Memorials**: `StorageMap<u32, MemorialInfo>`
   - 存储所有祭念馆的详细信息，包括 IPFS 文件列表

2. **NextMemorialId**: `StorageValue<u32>`
   - 下一个可用的祭念馆ID

3. **AccountMemorials**: `StorageMap<AccountId, BoundedVec<u32>>`
   - 每个账户拥有的祭念馆ID列表

4. **CeremonyRecords**: `StorageDoubleMap<u32, u32, CeremonyRecord>`
   - 祭奠活动记录，包含附件文件信息

5. **MemorialRecordCount**: `StorageMap<u32, u32>`
   - 每个祭念馆的祭奠记录数量

## 事件类型

```rust
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
```

## 可调用函数

### 1. create_memorial
创建新的祭念馆

**参数**：
- `name: Vec<u8>` - 逝者姓名
- `birth_date: Option<Vec<u8>>` - 生日（可选）
- `death_date: Option<Vec<u8>>` - 逝世日期（可选）
- `description: Vec<u8>` - 纪念描述
- `is_public: bool` - 是否公开访问
- `avatar_ipfs: Option<Vec<u8>>` - 头像照片的 IPFS 哈希（可选）

**功能**：为逝者创建专属的链上祭念馆，支持设置头像照片

### 2. add_memorial_media
为祭念馆添加媒体文件

**参数**：
- `memorial_id: u32` - 祭念馆ID
- `files: Vec<IpfsFile>` - 要添加的 IPFS 文件列表

**功能**：添加图片、音频、视频等媒体文件到祭念馆相册

### 3. perform_traditional_ceremony
进行传统祭奠活动（支持媒体附件）

**参数**：
- `memorial_id: u32` - 祭念馆ID
- `ceremony: TraditionalCeremony` - 传统祭奠类型
- `message: Vec<u8>` - 祭奠留言
- `karma_amount: Option<KarmaBalance>` - 可选的自定义Karma消费量
- `attached_files: Vec<IpfsFile>` - 附加的媒体文件

**支持的传统祭奠类型**：
- `OfferFlowers` - 献花（可附加花束照片）
- `LightCandles` - 点烛（可附加烛光视频）
- `BurnIncense` - 烧香（可附加祭香照片）
- `OfferFood` - 供奉食物（可附加供品照片）
- `BurnPaperMoney` - 烧纸钱（可附加祭祀照片）
- `Kowtow` - 磕头拜祭（可附加祭拜视频）

### 4. perform_modern_ceremony
进行现代纪念活动（支持媒体附件）

**参数**：
- `memorial_id: u32` - 祭念馆ID
- `ceremony: ModernCeremony` - 现代纪念类型
- `message: Vec<u8>` - 纪念留言
- `karma_amount: Option<KarmaBalance>` - 可选的自定义Karma消费量
- `attached_files: Vec<IpfsFile>` - 附加的媒体文件

**支持的现代纪念类型**：
- `OfferFlowers` - 献花（可附加鲜花照片）
- `LightPrayerLamps` - 点祈福灯（可附加祈福视频）
- `WriteMessage` - 写留言（可附加手写信件）
- `PlayMusic` - 播放音乐（可附加音频文件）
- `ShareMemories` - 分享回忆（可附加回忆照片/视频）
- `OnlineMemorial` - 在线追思（可附加追思文档）

## 前端集成示例

### IPFS 文件上传
```javascript
// 上传文件到 IPFS
const uploadToIpfs = async (file) => {
  const formData = new FormData();
  formData.append('file', file);
  
  const response = await fetch('/api/ipfs/upload', {
    method: 'POST',
    body: formData
  });
  
  const result = await response.json();
  return {
    hash: result.hash,
    file_type: file.type,
    size: file.size,
    name: file.name
  };
};

// 创建带头像的祭念馆
const createMemorialWithAvatar = async (name, description, avatarFile) => {
  let avatarIpfs = null;
  
  if (avatarFile) {
    avatarIpfs = await uploadToIpfs(avatarFile);
  }
  
  const tx = api.tx.commemorate.createMemorial(
    name, null, null, description, true, avatarIpfs?.hash
  );
  return await tx.signAndSend(keyring);
};

// 进行带媒体附件的祭奠
const ceremonyWithMedia = async (memorialId, ceremony, message, mediaFiles) => {
  const attachedFiles = [];
  
  for (const file of mediaFiles) {
    const ipfsFile = await uploadToIpfs(file);
    attachedFiles.push(ipfsFile);
  }
  
  const tx = api.tx.commemorate.performTraditionalCeremony(
    memorialId, ceremony, message, null, attachedFiles
  );
  return await tx.signAndSend(keyring);
};
```

### 媒体文件展示
```javascript
// 从 IPFS 获取文件 URL
const getIpfsUrl = (hash) => {
  return `https://ipfs.io/ipfs/${hash}`;
};

// 展示祭念馆相册
const displayGallery = (memorial) => {
  memorial.gallery_files.forEach(file => {
    const url = getIpfsUrl(file.hash);
    
    if (file.file_type.startsWith('image/')) {
      // 显示图片
      const img = document.createElement('img');
      img.src = url;
      img.alt = file.name;
    } else if (file.file_type.startsWith('audio/')) {
      // 播放音频
      const audio = document.createElement('audio');
      audio.src = url;
      audio.controls = true;
    } else if (file.file_type.startsWith('video/')) {
      // 播放视频
      const video = document.createElement('video');
      video.src = url;
      video.controls = true;
    }
  });
};
```

## 安全与隐私

### 访问控制
- 祭念馆创建者拥有完全管理权限
- 支持公开和私密两种访问模式
- 私密祭念馆仅创建者和被授权用户可访问

### IPFS 安全考虑
- 文件哈希验证确保内容完整性
- 支持多个 IPFS 网关提高可用性
- 建议对敏感内容进行客户端加密

### Karma 防护
- 所有操作都需要消耗 Karma，防止垃圾数据
- 基于操作类型设置不同的 Karma 消费标准
- 支持自定义 Karma 数量表达诚意程度

## 性能优化

### 存储优化
- 使用 `BoundedVec` 限制存储大小
- 大文件存储在 IPFS，链上仅存储元数据
- 分层存储设计减少链上存储压力

### 查询优化
- 使用双重映射优化祭奠记录查询
- 支持按时间、类型等维度筛选
- 提供批量查询接口提升效率

本模块将传统祭奠文化与区块链技术、分布式存储完美结合，为用户提供一个永久、安全、富媒体的数字纪念平台。