package land.buddha.app.core.metadata

import land.buddha.app.core.rpc.RpcService

/**
 * 函数级中文注释：
 * RuntimeMetadata 提供对 state_getMetadata 的解析占位：
 * - 真实实现需解析 SCALE MetadataV14/V15，获取 pallet/call 索引与类型信息。
 * - 由于解析较为复杂，此处给出占位结构与接口，便于后续替换为成熟实现。
 */
class RuntimeMetadata(private val rpc: RpcService) {

    data class PalletCallIndex(val palletIndex: Int, val callIndex: Int)

    /**
     * 函数级中文注释：
     * fetchRawMetadata 直接获取 HEX 字符串，用于离线解析或缓存；
     * 目前先返回原始字符串。
     */
    suspend fun fetchRawMetadata(): String = rpc.stateGetMetadata()

    /**
     * 函数级中文注释：
     * resolveCallIndex 根据 pallet 名与 call 名解析索引（占位实现）。
     * 真实实现应根据 Metadata pallets[...].calls[...].index 返回正确值。
     */
    suspend fun resolveCallIndex(pallet: String, call: String): PalletCallIndex? {
        // TODO: 解析 metadata，当前返回 null 以提示调用方传入显式索引
        return null
    }
}


