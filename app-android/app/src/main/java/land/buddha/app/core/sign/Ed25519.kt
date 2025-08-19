package land.buddha.app.core.sign

import org.bouncycastle.crypto.params.Ed25519PrivateKeyParameters
import org.bouncycastle.crypto.signers.Ed25519Signer

/**
 * 函数级中文注释：
 * Ed25519Keypair 通过 32 字节私钥种子（Mini-Secret）构造，
 * 提供获取公钥与对 payload 进行 Ed25519 签名的方法。
 */
class Ed25519Keypair(private val privateSeed32: ByteArray) {
    init { require(privateSeed32.size == 32) { "Ed25519 私钥种子需为 32 字节" } }

    /**
     * 函数级中文注释：
     * publicKey 返回 32 字节公钥。
     */
    fun publicKey(): ByteArray = Ed25519PrivateKeyParameters(privateSeed32, 0).generatePublicKey().encoded

    /**
     * 函数级中文注释：
     * sign 对 payload 执行 Ed25519 签名，返回 64 字节签名。
     */
    fun sign(payload: ByteArray): ByteArray {
        val priv = Ed25519PrivateKeyParameters(privateSeed32, 0)
        val signer = Ed25519Signer()
        signer.init(true, priv)
        signer.update(payload, 0, payload.size)
        return signer.generateSignature()
    }
}


