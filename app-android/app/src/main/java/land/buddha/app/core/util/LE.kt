package land.buddha.app.core.util

/**
 * 函数级中文注释：
 * 提供无符号整数的 Little-Endian 编码辅助函数（u32/u64）。
 */
object LE {
    /**
     * 函数级中文注释：
     * 将 32 位无符号整数（以 Kotlin Long 表示，范围 0..2^32-1）编码为 4 字节小端序。
     */
    fun u32(value: Long): ByteArray {
        require(value in 0..0xFFFF_FFFFL)
        return byteArrayOf(
            (value and 0xFF).toByte(),
            ((value ushr 8) and 0xFF).toByte(),
            ((value ushr 16) and 0xFF).toByte(),
            ((value ushr 24) and 0xFF).toByte(),
        )
    }

    /**
     * 函数级中文注释：
     * 将 64 位无符号整数（以 Kotlin Long 表示，非负）编码为 8 字节小端序。
     */
    fun u64(value: Long): ByteArray {
        require(value >= 0)
        return byteArrayOf(
            (value and 0xFF).toByte(),
            ((value ushr 8) and 0xFF).toByte(),
            ((value ushr 16) and 0xFF).toByte(),
            ((value ushr 24) and 0xFF).toByte(),
            ((value ushr 32) and 0xFF).toByte(),
            ((value ushr 40) and 0xFF).toByte(),
            ((value ushr 48) and 0xFF).toByte(),
            ((value ushr 56) and 0xFF).toByte(),
        )
    }
}


