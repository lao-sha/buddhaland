package land.buddha.app.utils

/**
 * 函数级中文注释：
 * 从十六进制字符串（可带 0x 前缀）解析为 Long，
 * 常用于解析区块头中 number 字段（hex 表示）。
 */
fun hexToLong(hex: String): Long {
    val s = if (hex.startsWith("0x") || hex.startsWith("0X")) hex.substring(2) else hex
    if (s.isEmpty()) return 0L
    return s.toULong(radix = 16).toLong()
}

/**
 * 函数级中文注释：
 * hexToBytes 将十六进制字符串（可带 0x 前缀）转换为字节数组（大写/小写均可）。
 */
fun hexToBytes(hex: String): ByteArray {
    val s = if (hex.startsWith("0x") || hex.startsWith("0X")) hex.substring(2) else hex
    require(s.length % 2 == 0) { "Hex 长度必须为偶数" }
    val out = ByteArray(s.length / 2)
    var i = 0
    while (i < s.length) {
        out[i / 2] = ((s[i].digitToInt(16) shl 4) + s[i + 1].digitToInt(16)).toByte()
        i += 2
    }
    return out
}

/**
 * 函数级中文注释：
 * bytesToHex 将字节数组转换为 0x 前缀的小写十六进制字符串。
 */
fun bytesToHex(bytes: ByteArray): String {
    val sb = StringBuilder(bytes.size * 2 + 2)
    sb.append("0x")
    for (b in bytes) {
        val v = b.toInt() and 0xFF
        sb.append("0123456789abcdef"[v ushr 4])
        sb.append("0123456789abcdef"[v and 0x0F])
    }
    return sb.toString()
}


