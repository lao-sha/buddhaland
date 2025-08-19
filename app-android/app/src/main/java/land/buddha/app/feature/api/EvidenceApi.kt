package land.buddha.app.feature.api

import land.buddha.app.core.rpc.RpcService

/**
 * 函数级中文注释：
 * EvidenceApi 封装与 pallet-evidence 相关的 extrinsic 提交流程。
 * 当前阶段不构造交易，仅提交外部已签名的 hex。
 */
class EvidenceApi(private val rpc: RpcService) {
    /**
     * 函数级中文注释：
     * commit 占位：提交证据；直接提交已签名交易 hex。
     */
    suspend fun commit(signedExtrinsicHex: String): String = rpc.authorSubmitExtrinsic(signedExtrinsicHex)

    /**
     * 函数级中文注释：
     * link 占位：链接证据；直接提交已签名交易 hex。
     */
    suspend fun link(signedExtrinsicHex: String): String = rpc.authorSubmitExtrinsic(signedExtrinsicHex)

    /**
     * 函数级中文注释：
     * unlink 占位：取消链接证据；直接提交已签名交易 hex。
     */
    suspend fun unlink(signedExtrinsicHex: String): String = rpc.authorSubmitExtrinsic(signedExtrinsicHex)
}


