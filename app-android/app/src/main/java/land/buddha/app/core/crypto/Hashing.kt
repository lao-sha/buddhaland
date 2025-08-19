package land.buddha.app.core.crypto

import org.bouncycastle.crypto.digests.Blake2bDigest

/**
 * 函数级中文注释：
 * Hashing 提供常用哈希函数封装，目前实现 blake2b-256，
 * 用于签名 payload 的长度裁剪与交易签名流程。
 */
object Hashing {
	/**
     * 函数级中文注释：
     * blake2b256 计算输入的 blake2b-256 摘要并返回 32 字节数组。
     */
    fun blake2b256(input: ByteArray): ByteArray {
        val d = Blake2bDigest(256)
        d.update(input, 0, input.size)
        val out = ByteArray(32)
        d.doFinal(out, 0)
        return out
    }
}


