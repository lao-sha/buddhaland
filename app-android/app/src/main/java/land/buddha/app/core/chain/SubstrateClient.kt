package land.buddha.app.core.chain

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import land.buddha.app.core.rpc.ChainHeader
import land.buddha.app.core.rpc.JsonRpcRequest
import land.buddha.app.core.ws.WsClient
import land.buddha.app.utils.hexToLong

/**
 * 函数级中文注释：
 * SubstrateClient 提供基于 JSON-RPC 的最小封装：
 * - 获取最新区块高度
 * - 发送自定义 extrinsic（占位：需引入 SCALE 编码与签名）
 * 实际生产中应引入成熟 SDK（如 fearless/nova 组件）或自行封装 SCALE/签名。
 */
class SubstrateClient(private val ws: WsClient) {
    private val json = Json { ignoreUnknownKeys = true }

    /**
     * 函数级中文注释：
     * fetchBestBlock 查询最新区块头并解析高度。
     */
    suspend fun fetchBestBlock(): Long = withContext(Dispatchers.IO) {
        val id = nextId()
        val req = JsonRpcRequest<List<String>>(id = id, method = "chain_getHeader", params = emptyList())
        val text = ws.sendForResult(id, json.encodeToString(req))
        val element = json.parseToJsonElement(text)
        val obj = element.jsonObject
        val result = obj["result"] as? JsonObject
        val header = result?.let { json.decodeFromJsonElement(ChainHeader.serializer(), it) }
        header?.numberHex?.let { hexToLong(it) } ?: 0L
    }

    /**
     * 函数级中文注释：
     * submitExtrinsic 发送签名后的 extrinsic（hex 编码），返回交易 hash。
     * 这里作为占位实现，直接走 author_submitExtrinsic；
     * 注意：构造签名 payload/nonce/era/签名曲线需要 SCALE 支持，后续补充。
     */
    suspend fun submitExtrinsic(signedExtrinsicInHex: String): String = withContext(Dispatchers.IO) {
        val id = nextId()
        val req = JsonRpcRequest<List<String>>(id = id, method = "author_submitExtrinsic", params = listOf(signedExtrinsicInHex))
        val text = ws.sendForResult(id, json.encodeToString(req))
        val element = json.parseToJsonElement(text).jsonObject
        element["result"]?.jsonPrimitive?.content ?: ""
    }

    @Volatile private var counter: Int = 1
    /**
     * 函数级中文注释：
     * nextId 生成自增请求 id，保证并发调用时的唯一性。
     */
    private fun nextId(): Int = synchronized(this) { counter++ }
}


