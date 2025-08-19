package land.buddha.app.core.scale

import java.io.ByteArrayOutputStream

/**
 * 函数级中文注释：
 * 仅实现最小 SCALE Compact<u128>/u32 编码与 Bytes 编码，
 * 便于演示构造 extrinsic 的 payload（不含完整 Metadata 类型支持）。
 */
object ScaleCodec {
    /**
     * 函数级中文注释：
     * 编码 u32 为 SCALE Compact 形式。
     */
    fun compactU32(value: Long): ByteArray {
        require(value >= 0)
        return when {
            value < 1 shl 6 -> byteArrayOf((value shl 2).toByte())
            value < 1L shl 14 -> {
                val v = (value shl 2) or 0x01
                byteArrayOf((v and 0xFF).toByte(), ((v ushr 8) and 0xFF).toByte())
            }
            value < 1L shl 30 -> {
                val v = (value shl 2) or 0x02
                byteArrayOf(
                    (v and 0xFF).toByte(),
                    ((v ushr 8) and 0xFF).toByte(),
                    ((v ushr 16) and 0xFF).toByte(),
                    ((v ushr 24) and 0xFF).toByte(),
                )
            }
            else -> {
                val bytes = encodeLE(value)
                val lenPrefix = ((bytes.size - 4) shl 2) or 0x03
                ByteArrayOutputStream().apply {
                    write(lenPrefix)
                    write(bytes)
                }.toByteArray()
            }
        }
    }

    /**
     * 函数级中文注释：
     * 编码原始字节数组（先写 Compact 长度，再写数据）。
     */
    fun bytes(raw: ByteArray): ByteArray {
        val len = compactU32(raw.size.toLong())
        return ByteArrayOutputStream().apply {
            write(len)
            write(raw)
        }.toByteArray()
    }

    private fun encodeLE(value: Long): ByteArray {
        // 最多用到 u32 范围
        return byteArrayOf(
            (value and 0xFF).toByte(),
            ((value ushr 8) and 0xFF).toByte(),
            ((value ushr 16) and 0xFF).toByte(),
            ((value ushr 24) and 0xFF).toByte()
        )
    }
}


