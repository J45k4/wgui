package fi.puppycorp.wgui.renderer

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import java.util.concurrent.TimeUnit

class WguiClient(
    private val serverUrl: String,
    private val onMessages: (List<SrvMessage>) -> Unit
) {
    private val scope = CoroutineScope(Dispatchers.Main)
    private val client = OkHttpClient.Builder()
        .readTimeout(0, TimeUnit.MILLISECONDS)
        .build()
    private var webSocket: WebSocket? = null
    private var currentPath: String = "/"
    private var currentQuery: Map<String, String> = emptyMap()

    fun connect() {
        val wsUrl = toWebSocketUrl(serverUrl)
        val request = Request.Builder().url(wsUrl).build()
        webSocket = client.newWebSocket(request, object : WebSocketListener() {
            override fun onOpen(webSocket: WebSocket, response: Response) {
                sendPathChanged(currentPath, currentQuery)
            }

            override fun onMessage(webSocket: WebSocket, text: String) {
                val messages = parseSrvMessages(text)
                for (message in messages) {
                    when (message) {
                        is SrvMessage.PushState -> {
                            currentPath = message.url
                            sendPathChanged(currentPath, currentQuery)
                        }
                        is SrvMessage.ReplaceState -> {
                            currentPath = message.url
                        }
                        is SrvMessage.SetQuery -> {
                            currentQuery = message.query
                        }
                        else -> {}
                    }
                }
                onMessages(messages)
            }

            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                scheduleReconnect()
            }

            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                scheduleReconnect()
            }
        })
    }

    fun close() {
        webSocket?.close(1000, "closed")
        webSocket = null
    }

    fun sendPathChanged(path: String, query: Map<String, String>) {
        currentPath = path
        currentQuery = query
        val queryObj = JsonObject(query.mapValues { JsonPrimitive(it.value) })
        sendMessage(
            JsonObject(
                mapOf(
                    "type" to JsonPrimitive("pathChanged"),
                    "path" to JsonPrimitive(path),
                    "query" to queryObj
                )
            )
        )
    }

    fun sendOnClick(id: Int, inx: Int?) {
        val base = mutableMapOf<String, JsonPrimitive>(
            "type" to JsonPrimitive("onClick"),
            "id" to JsonPrimitive(id)
        )
        if (inx != null) {
            base["inx"] = JsonPrimitive(inx)
        }
        sendMessage(JsonObject(base))
    }

    fun sendOnTextChanged(id: Int, inx: Int?, value: String) {
        val base = mutableMapOf<String, JsonPrimitive>(
            "type" to JsonPrimitive("onTextChanged"),
            "id" to JsonPrimitive(id),
            "value" to JsonPrimitive(value)
        )
        if (inx != null) {
            base["inx"] = JsonPrimitive(inx)
        }
        sendMessage(JsonObject(base))
    }

    fun sendOnSliderChange(id: Int, inx: Int?, value: Int) {
        val base = mutableMapOf<String, JsonPrimitive>(
            "type" to JsonPrimitive("onSliderChange"),
            "id" to JsonPrimitive(id),
            "value" to JsonPrimitive(value)
        )
        if (inx != null) {
            base["inx"] = JsonPrimitive(inx)
        }
        sendMessage(JsonObject(base))
    }

    fun sendOnSelect(id: Int, inx: Int?, value: String) {
        val base = mutableMapOf<String, JsonPrimitive>(
            "type" to JsonPrimitive("onSelect"),
            "id" to JsonPrimitive(id),
            "value" to JsonPrimitive(value)
        )
        if (inx != null) {
            base["inx"] = JsonPrimitive(inx)
        }
        sendMessage(JsonObject(base))
    }

    private fun sendMessage(message: JsonObject) {
        val socket = webSocket ?: return
        val payload = JsonArray(listOf(message)).toString()
        socket.send(payload)
    }

    private fun scheduleReconnect() {
        scope.launch {
            delay(1000)
            connect()
        }
    }

    private fun toWebSocketUrl(server: String): String {
        val trimmed = server.trim().removeSuffix("/")
        return if (trimmed.startsWith("https://")) {
            "wss://" + trimmed.removePrefix("https://") + "/ws"
        } else if (trimmed.startsWith("http://")) {
            "ws://" + trimmed.removePrefix("http://") + "/ws"
        } else {
            "ws://$trimmed/ws"
        }
    }
}
