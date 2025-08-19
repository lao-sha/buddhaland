package land.buddha.app.core.rpc

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import land.buddha.app.core.ws.WsClient

/**
 * 函数级中文注释：
 * RpcService 提供常用 JSON-RPC 便捷封装：
 * - 查询账户下一个 nonce（system_accountNextIndex）
 * - 预估交易费用（payment_queryInfo）
 * - 直接提交已签名交易（author_submitExtrinsic）
 */
class RpcService(private val ws: WsClient) {
    private val json = Json { ignoreUnknownKeys = true }

    /**
     * 函数级中文注释：
     * 获取账户下一个 nonce；兼容 result 可能为数字或十六进制字符串的情况。
     */
    suspend fun systemAccountNextIndex(accountIdHex: String): Long = withContext(Dispatchers.IO) {
        val id = nextId()
        val req = JsonRpcRequest<List<String>>(id = id, method = "system_accountNextIndex", params = listOf(accountIdHex))
        val text = ws.sendForResult(id, json.encodeToString(req))
        val element = json.parseToJsonElement(text).jsonObject
        val result = element["result"]
        when {
            result == null -> 0L
            result.jsonPrimitive.isString -> result.jsonPrimitive.content.toLongOrNull() ?: hexToLong(result.jsonPrimitive.content)
            else -> result.jsonPrimitive.long
        }
    }

    /**
     * 函数级中文注释：
     * 预估已签名交易的费用信息，返回部分 Fee 字段以便展示或风控。
     */
    suspend fun paymentQueryInfo(extrinsicHex: String): PaymentInfo = withContext(Dispatchers.IO) {
        val id = nextId()
        val req = JsonRpcRequest<List<String>>(id = id, method = "payment_queryInfo", params = listOf(extrinsicHex))
        val text = ws.sendForResult(id, json.encodeToString(req))
        val result = json.parseToJsonElement(text).jsonObject["result"] as JsonObject
        json.decodeFromJsonElement(PaymentInfo.serializer(), result)
    }

    /**
     * 函数级中文注释：
     * 直接提交已签名交易，返回交易哈希（hex）。
     */
    suspend fun authorSubmitExtrinsic(extrinsicHex: String): String = withContext(Dispatchers.IO) {
        val id = nextId()
        val req = JsonRpcRequest<List<String>>(id = id, method = "author_submitExtrinsic", params = listOf(extrinsicHex))
        val text = ws.sendForResult(id, json.encodeToString(req))
        json.parseToJsonElement(text).jsonObject["result"]?.jsonPrimitive?.content ?: ""
    }

    /**
     * 函数级中文注释：
     * 获取链上 Metadata（SCALE 编码的 hex 字符串），
     * 供后续解析类型与构造 extrinsic 使用。
     */
    suspend fun stateGetMetadata(): String = withContext(Dispatchers.IO) {
        val id = nextId()
        val req = JsonRpcRequest<List<String>>(id = id, method = "state_getMetadata", params = emptyList())
        val text = ws.sendForResult(id, json.encodeToString(req))
        json.parseToJsonElement(text).jsonObject["result"]?.jsonPrimitive?.content ?: ""
    }

    /**
     * 函数级中文注释：
     * 获取 genesis 区块哈希（用于签名 payload 的附加字段）。
     */
    suspend fun chainGetBlockHash(height: Long = 0): String = withContext(Dispatchers.IO) {
        val id = nextId()
        val params = if (height == 0L) emptyList() else listOf(height.toString())
        val req = JsonRpcRequest<List<String>>(id = id, method = "chain_getBlockHash", params = params)
        val text = ws.sendForResult(id, json.encodeToString(req))
        json.parseToJsonElement(text).jsonObject["result"]?.jsonPrimitive?.content ?: ""
    }

    /**
     * 函数级中文注释：
     * 获取 runtime 版本信息（specVersion/transactionVersion）。
     */
    suspend fun stateGetRuntimeVersion(): RuntimeVersion = withContext(Dispatchers.IO) {
        val id = nextId()
        val req = JsonRpcRequest<List<String>>(id = id, method = "state_getRuntimeVersion", params = emptyList())
        val text = ws.sendForResult(id, json.encodeToString(req))
        val result = json.parseToJsonElement(text).jsonObject["result"] as JsonObject
        json.decodeFromJsonElement(RuntimeVersion.serializer(), result)
    }

    @Volatile private var counter: Int = 1000
    private fun nextId(): Int = synchronized(this) { ++counter }
}
@Serializable
data class RuntimeVersion(
    @SerialName("specVersion") val specVersion: Long,
    @SerialName("transactionVersion") val transactionVersion: Long
)


@Serializable
data class PaymentInfo(
    @SerialName("partialFee") val partialFee: String,
    @SerialName("weight") val weight: PaymentWeight,
    @SerialName("class") val clazz: String? = null
)

@Serializable
data class PaymentWeight(
    @SerialName("refTime") val refTime: String,
    @SerialName("proofSize") val proofSize: String
)

/**
 * 函数级中文注释：
 * 工具函数：解析可能的十六进制字符串为 Long。
 */
private fun hexToLong(hex: String): Long {
    val s = if (hex.startsWith("0x") || hex.startsWith("0X")) hex.substring(2) else hex
    if (s.isEmpty()) return 0L
    return s.toULong(radix = 16).toLong()
}


