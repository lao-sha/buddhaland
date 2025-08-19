package land.buddha.app.core

import land.buddha.app.core.chain.SubstrateClient
import land.buddha.app.core.rpc.RpcService
import land.buddha.app.core.ws.WsClient
import land.buddha.app.feature.api.ArbitrationApi
import land.buddha.app.feature.api.EvidenceApi
import land.buddha.app.feature.api.ExchangeApi
import land.buddha.app.feature.api.OrderApi

/**
 * 函数级中文注释：
 * AppDeps 负责集中管理应用所需依赖（WS/RPC/API 实例），
 * 提供统一的创建、访问与释放方法，避免在 UI 层反复初始化。
 */
object AppDeps {
    // 如需切换网络，可改为可变配置或从偏好读取
    private const val DEFAULT_WS = "ws://10.0.2.2:9944"

    private val wsClient: WsClient by lazy { WsClient(DEFAULT_WS) }
    private val rpcService: RpcService by lazy { RpcService(wsClient) }
    private val substrate: SubstrateClient by lazy { SubstrateClient(wsClient) }

    val orderApi: OrderApi by lazy { OrderApi(rpcService, signer = nullSigner()) }
    val evidenceApi: EvidenceApi by lazy { EvidenceApi(rpcService) }
    val arbitrationApi: ArbitrationApi by lazy { ArbitrationApi(rpcService) }
    val exchangeApi: ExchangeApi by lazy { ExchangeApi(rpcService) }

    /**
     * 函数级中文注释：
     * connectIfNeeded 用于在 UI 触发时建立 WS 连接；
     * 多次调用是安全的（内部会先断开再连）。
     */
    fun connectIfNeeded() {
        wsClient.connect()
    }

    /**
     * 函数级中文注释：
     * substrateClient 暴露底层 SubstrateClient，便于查询高度或提交交易。
     */
    fun substrateClient(): SubstrateClient = substrate

    /**
     * 函数级中文注释：
     * shutdown 关闭 WS 连接并清理资源。
     */
    fun shutdown() {
        wsClient.disconnect()
    }

    // 占位：返回一个不具备签名能力的 signer（仅为 API 构造形参占位）
    private fun nullSigner() = object : land.buddha.app.core.sign.RemoteSigner {
        override fun sign(payload: ByteArray, accountUri: String, nonce: Long) =
            throw UnsupportedOperationException("未集成签名器，请使用外部已签名的 extrinsic hex")
    }
}


