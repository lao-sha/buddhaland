package land.buddha.app.feature.api

import land.buddha.app.core.rpc.RpcService

/**
 * 函数级中文注释：
 * ArbitrationApi 封装与 pallet-arbitration 相关的 extrinsic 提交流程。
 * 当前阶段不构造交易，仅提交外部已签名的 hex。
 */
class ArbitrationApi(private val rpc: RpcService) {
    /**
     * 函数级中文注释：
     * dispute 占位：发起仲裁；直接提交已签名交易 hex。
     */
    suspend fun dispute(signedExtrinsicHex: String): String = rpc.authorSubmitExtrinsic(signedExtrinsicHex)

    /**
     * 函数级中文注释：
     * arbitrate 占位：提交裁决；直接提交已签名交易 hex。
     */
    suspend fun arbitrate(signedExtrinsicHex: String): String = rpc.authorSubmitExtrinsic(signedExtrinsicHex)
}


