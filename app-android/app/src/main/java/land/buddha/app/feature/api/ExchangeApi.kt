package land.buddha.app.feature.api

import land.buddha.app.core.crypto.Hashing
import land.buddha.app.core.extrinsic.ExtrinsicBuilder
import land.buddha.app.core.extrinsic.SignedExtrinsicBuilder
import land.buddha.app.core.rpc.RpcService
import land.buddha.app.utils.hexToBytes

/**
 * 函数级中文注释：
 * ExchangeApi 封装与 pallet-exchange 相关的 extrinsic 构造与提交流程：
 * - 支持直接提交外部已签名 hex
 * - 提供基于“已知 pallet/call 索引 + Ed25519 签名”的极简签名示例
 */
class ExchangeApi(private val rpc: RpcService) {
    /**
     * 函数级中文注释：
     * exchange 提交已签名交易 hex（外部签名场景）。
     */
    suspend fun exchange(signedExtrinsicHex: String): String = rpc.authorSubmitExtrinsic(signedExtrinsicHex)

    /**
     * 函数级中文注释：
     * signAndSubmitExchange 演示：
     * - 通过已知 palletIndex/callIndex 构造 call bytes
     * - 组装极简签名 payload（call + nonce + genesisHash）
     * - 调用外部 Ed25519 签名器（由调用方提供），得到 64 字节签名
     * - 封装为极简 signed extrinsic 并提交
     * 注意：生产中请基于 Metadata 与 MultiSignature 正确构造版本/era/specVersion 等。
     */
    suspend fun signAndSubmitExchange(
        palletIndex: Int,
        callIndex: Int,
        budAmount: Long,
        nonce: Long,
        genesisHashHex: String,
        publicKey32: ByteArray,
        ed25519Signer: (payload: ByteArray) -> ByteArray
    ): String {
        val call = ExtrinsicBuilder.buildExchangeCall(palletIndex, callIndex, budAmount)
        val payload = SignedExtrinsicBuilder.buildSignPayload(call, nonce, hexToBytes(genesisHashHex))
        val sig = ed25519Signer(payload)
        require(sig.size == 64) { "Ed25519 签名长度应为 64 字节" }
        val extrinsicHex = SignedExtrinsicBuilder.wrapSignedExtrinsic(call, publicKey32, sig)
        return rpc.authorSubmitExtrinsic(extrinsicHex)
    }
}


