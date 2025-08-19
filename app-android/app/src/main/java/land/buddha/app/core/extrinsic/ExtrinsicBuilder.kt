package land.buddha.app.core.extrinsic

import land.buddha.app.core.scale.ScaleCodec
import land.buddha.app.utils.bytesToHex

/**
 * 函数级中文注释：
 * ExtrinsicBuilder 提供极简的“未签名 call 数据”构造：
 * - 仅支持传入 palletIndex、callIndex 与单个 Compact<u128> 金额参数（演示 exchange(bud_amount)）
 * - 真实项目应根据 Metadata 动态编码参数类型与数量，这里仅作演示
 */
object ExtrinsicBuilder {
    /**
     * 函数级中文注释：
     * buildExchangeCall 组装 exchange(bud_amount) 的 call bytes：
     * | pallet_index (u8) | call_index (u8) | amount(Compact<u128> 简化为 u32 范围) |
     */
    fun buildExchangeCall(palletIndex: Int, callIndex: Int, budAmount: Long): ByteArray {
        require(palletIndex in 0..255 && callIndex in 0..255)
        val amount = ScaleCodec.compactU32(budAmount)
        return byteArrayOf(palletIndex.toByte(), callIndex.toByte()) + amount
    }

    /**
     * 函数级中文注释：
     * buildUnsigned 从 call bytes 生成“未签名的交易 payload 占位”（直接返回 call 本体），
     * 真正的签名需要 era/nonce/tip/genesisHash/specVersion 等参与构造；
     * 这里为方便演示，先走“外部已签名 hex”路径。
     */
    fun buildUnsigned(callBytes: ByteArray): String = bytesToHex(callBytes)
}


