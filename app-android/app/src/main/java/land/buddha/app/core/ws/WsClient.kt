package land.buddha.app.core.ws

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.decodeFromJsonElement
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import okio.ByteString
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.TimeUnit

/**
 * 函数级中文注释：
 * WsClient 封装 OkHttp WebSocket 连接与请求-响应匹配逻辑。
 * - 提供 connect/disconnect 生命周期管理
 * - 提供 sendRaw 与 sendForResult 两种发送方式
 * - 基于请求 id（Int）将 JSON-RPC 响应分发回等待的调用者
 */
class WsClient(private val url: String) {

    private val json = Json { ignoreUnknownKeys = true }

    private val client: OkHttpClient = OkHttpClient.Builder()
        .pingInterval(15, TimeUnit.SECONDS)
        .readTimeout(0, TimeUnit.MILLISECONDS)
        .build()

    private var webSocket: WebSocket? = null
    private val scope = CoroutineScope(Dispatchers.IO + Job())

    private val pending = ConcurrentHashMap<Int, CompletableDeferred<String>>()

    /**
     * 函数级中文注释：
     * connect 建立 WebSocket 连接；如已存在连接则先断开再重连。
     */
    fun connect() {
        disconnect()
        val request = Request.Builder().url(url).build()
        webSocket = client.newWebSocket(request, object : WebSocketListener() {
            override fun onOpen(webSocket: WebSocket, response: Response) {
                // 连接成功回调，可在此上报状态
            }

            override fun onMessage(webSocket: WebSocket, text: String) {
                scope.launch { handleMessage(text) }
            }

            override fun onMessage(webSocket: WebSocket, bytes: ByteString) {
                // 节点一般返回文本 JSON
            }

            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                // 连接关闭，清理未完成请求
                failAll("WebSocket closed: $code/$reason")
            }

            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                failAll("WebSocket failure: ${t.message}")
            }
        })
    }

    /**
     * 函数级中文注释：
     * disconnect 主动断开连接，并取消所有挂起请求。
     */
    fun disconnect() {
        try {
            webSocket?.close(1000, "normal")
        } catch (_: Throwable) {
        } finally {
            webSocket = null
            failAll("Disconnected")
        }
    }

    /**
     * 函数级中文注释：
     * sendRaw 直接发送字符串消息；适用于订阅类或无需响应匹配的请求。
     */
    fun sendRaw(message: String): Boolean = webSocket?.send(message) ?: false

    /**
     * 函数级中文注释：
     * sendForResult 发送带 id 的 JSON-RPC 请求，并挂起等待返回的完整 JSON 文本。
     * 调用者随后可自行解析为所需结构。
     */
    suspend fun sendForResult(id: Int, message: String): String {
        val deferred = CompletableDeferred<String>()
        pending[id] = deferred
        val ok = webSocket?.send(message) ?: false
        if (!ok) {
            pending.remove(id)
            throw IllegalStateException("WebSocket not connected")
        }
        return deferred.await()
    }

    private fun failAll(reason: String) {
        // 函数级中文注释：
        // failAll 会在连接关闭或异常时，将所有挂起请求以异常形式完成，
        // 以避免调用方永久等待；随后清空待响应表并取消作用域。
        pending.forEach { (_, d) -> d.completeExceptionally(IllegalStateException(reason)) }
        pending.clear()
        scope.cancel()
    }

    private fun handleMessage(text: String) {
        // 函数级中文注释：
        // handleMessage 解析到达的文本消息，提取 JSON-RPC id；
        // 若为请求响应，则完成对应的挂起任务；否则忽略（可能是订阅通知）。
        // 尝试解析 id；若无 id，忽略（可能是订阅通知）
        runCatching {
            val element = json.parseToJsonElement(text)
            val obj = element.jsonObject
            val id = obj["id"]?.jsonPrimitive?.intOrNull
            if (id != null) {
                pending.remove(id)?.complete(text)
            }
        }
    }
}


