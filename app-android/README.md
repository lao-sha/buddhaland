# Buddha Land Android (Kotlin + Compose)

本目录为安卓原生客户端骨架，技术栈：Kotlin + Jetpack Compose + Coroutines。
已内置 JSON-RPC/WebSocket 封装，后续将补齐 SCALE/签名以对接链上 extrinsic 构造。

## 结构
- settings.gradle.kts / build.gradle.kts：根工程配置
- app/：应用模块
  - build.gradle.kts：模块配置（Compose/依赖）
  - src/main/AndroidManifest.xml：清单与权限
  - src/main/java/land/buddha/app/MainActivity.kt：入口界面
  - src/main/java/land/buddha/app/core/ws/WsClient.kt：WebSocket 客户端
  - src/main/java/land/buddha/app/core/rpc/*.kt：JSON-RPC 模型与常用 RPC 封装
  - src/main/java/land/buddha/app/core/chain/SubstrateClient.kt：区块高度/交易提交示例
  - src/main/java/land/buddha/app/core/sign/Signer.kt：签名器接口（占位）
  - src/main/java/land/buddha/app/feature/api/*.kt：订单/证据/仲裁/兑换 API 占位

## 开发
1. 安装 Android Studio（JDK 17）
2. 打开 app-android/ 作为工程，等待索引完成
3. 直接运行 app 模块（模拟器连接本机节点请使用 ws://10.0.2.2:9944）

## 后续计划
- 引入 Substrate Android SDK（或自研 SCALE/签名封装），包括：
  - WebSocket 连接（自定义网络）
  - 账户/签名（sr25519/ed25519）
  - 交易构造、签名与提交（author_submitExtrinsic）
  - 事件订阅与解码
- 按 pallets接口文档.md 对齐实现：订单/仲裁/证据/兑换等调用与界面
