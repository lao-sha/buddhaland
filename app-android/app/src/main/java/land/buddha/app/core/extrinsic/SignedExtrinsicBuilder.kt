package land.buddha.app.core.extrinsic

import land.buddha.app.core.crypto.Hashing
import land.buddha.app.core.scale.ScaleCodec
import land.buddha.app.core.util.LE
import land.buddha.app.utils.bytesToHex

/**
 * 函数级中文注释：
 * SignedExtrinsicBuilder 演示性地构造签名 extrinsic：
 * - 假设签名类型为 Ed25519（签名格式与 MultiSignature 需按 runtime 配置调整）
 * - 该实现为教学用途，真实项目需根据 metadata 组装完整 payload
 */
object SignedExtrinsicBuilder {

    /**
     * 函数级中文注释：
     * buildSignPayload 演示构造签名 payload（极简版）：
     * 拼接 callBytes + nonce(u32, LE) + genesisHash(32)
     * 长度超过 256 字节时需 blake2b-256 裁剪。
     */
    fun buildSignPayload(callBytes: ByteArray, nonce: Long, genesisHash: ByteArray): ByteArray {
        val base = callBytes + LE.u32(nonce) + genesisHash
        return if (base.size > 256) Hashing.blake2b256(base) else base
    }

    /**
     * 函数级中文注释：
     * wrapSignedExtrinsic 演示将签名、公钥与 call 封装为不带版本/era 的极简 extrinsic：
     * 实际需要 version/era/specVersion/transactionVersion/tip 等；此处仅供打通流程。
     */
    fun wrapSignedExtrinsic(callBytes: ByteArray, publicKey32: ByteArray, signature64: ByteArray): String {
        // 简化：0x84 表示签名版本占位（非真实），附带 signer 公钥与签名，再跟随 call
        val prefix = byteArrayOf(0x84.toByte())
        val body = prefix + publicKey32 + signature64 + callBytes
        val len = ScaleCodec.compactU32(body.size.toLong())
        return bytesToHex(len + body)
    }
}


