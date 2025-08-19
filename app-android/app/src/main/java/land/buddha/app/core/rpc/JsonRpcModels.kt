package land.buddha.app.core.rpc

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/**
 * 函数级中文注释：
 * JsonRpcRequest 定义标准 JSON-RPC 2.0 请求模型，
 * 通过范型 Params 支持任意参数结构，供 WebSocket 客户端序列化发送。
 */
@Serializable
data class JsonRpcRequest<Params>(
    @SerialName("jsonrpc") val jsonrpc: String = "2.0",
    @SerialName("id") val id: Int,
    @SerialName("method") val method: String,
    @SerialName("params") val params: Params
)

/**
 * 函数级中文注释：
 * JsonRpcResult 表示 JSON-RPC 成功响应的 result 字段，
 * 以范型 Result 承载任意结构并由上层解析。
 */
@Serializable
data class JsonRpcResult<Result>(
    @SerialName("jsonrpc") val jsonrpc: String = "2.0",
    @SerialName("id") val id: Int,
    @SerialName("result") val result: Result? = null,
    @SerialName("error") val error: JsonRpcError? = null
)

/**
 * 函数级中文注释：
 * JsonRpcError 对应 JSON-RPC 标准错误结构，
 * 用于在上层抛出或展示错误信息。
 */
@Serializable
data class JsonRpcError(
    @SerialName("code") val code: Int,
    @SerialName("message") val message: String,
    @SerialName("data") val data: String? = null
)

/**
 * 函数级中文注释：
 * ChainHeader 是对 chain_getHeader 返回体中关心字段的最小映射，
 * 这里只保留 "number"，用于解析最新区块高度。
 */
@Serializable
data class ChainHeader(
    @SerialName("number") val numberHex: String
)


