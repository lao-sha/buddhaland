package land.buddha.app.feature.api

import land.buddha.app.core.rpc.RpcService
import land.buddha.app.core.sign.RemoteSigner
import land.buddha.app.core.sign.SignedExtrinsic

/**
 * 函数级中文注释：
 * OrderApi 封装与 pallet-order 相关的 extrinsic 构造与提交流程。
 * 目前占位：构造签名 payload 的部分尚未实现，需后续接入 SCALE。
 */
class OrderApi(
    private val rpc: RpcService,
    private val signer: RemoteSigner
) {
    /**
     * 函数级中文注释：
     * createOrder 占位：创建订单。参数参考链上接口文档。
     * 现阶段仅演示签名交易的提交流程，入参为外部提供的已签名 hex。
     */
    suspend fun createOrder(signedExtrinsicHex: String): String {
        val signed = SignedExtrinsic(signedExtrinsicHex)
        return rpc.authorSubmitExtrinsic(signed.hex)
    }

    /**
     * 函数级中文注释：
     * acceptOrder 占位：代办人接受订单。现阶段同上，仅提交已签名 hex。
     */
    suspend fun acceptOrder(signedExtrinsicHex: String): String {
        return rpc.authorSubmitExtrinsic(signedExtrinsicHex)
    }

    /**
     * 函数级中文注释：
     * startOrder 占位：代办人开始订单。现阶段同上，仅提交已签名 hex。
     */
    suspend fun startOrder(signedExtrinsicHex: String): String {
        return rpc.authorSubmitExtrinsic(signedExtrinsicHex)
    }

    /**
     * 函数级中文注释：
     * submitOrderProof 占位：提交订单证据。现阶段同上，仅提交已签名 hex。
     */
    suspend fun submitOrderProof(signedExtrinsicHex: String): String {
        return rpc.authorSubmitExtrinsic(signedExtrinsicHex)
    }
}


