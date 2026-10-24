package com.codync.android

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.codync.android.core.WireJson
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.*

private val argumentJson = Json { prettyPrint = true }

@Serializable internal data class McpToolPolicy(val tool: String, val mode: String, val explicit: Boolean = true)
@Serializable internal data class McpToolRequest(
    val id: String, val botId: String, val tool: String, val status: String,
    val createdAt: Long, val expiresAt: Long,
)
@Serializable internal data class McpToolDetail(
    val id: String, val botId: String, val tool: String, val status: String, val arguments: JsonElement,
) {
    val formattedArguments: String get() = argumentJson.encodeToString(JsonElement.serializer(), arguments)
}
@Serializable internal data class McpReceipt(val status: String, val reply: JsonElement? = null, val id: String = "") {
    val formatted: String? get() = reply?.takeUnless { it == JsonNull }?.let { argumentJson.encodeToString(JsonElement.serializer(), it) }
}

/** Transient owner settings. The host validates scope, immutable arguments and expiry. */
internal class McpToolsModel(
    private val connectorId: String, private val store: AppStore,
    private val scope: CoroutineScope, private val generation: Int,
) {
    var policies by mutableStateOf<List<McpToolPolicy>>(emptyList()); private set
    var requests by mutableStateOf<List<McpToolRequest>>(emptyList()); private set
    var detail by mutableStateOf<McpToolDetail?>(null); private set
    var receipt by mutableStateOf<McpReceipt?>(null); private set
    var next by mutableStateOf<String?>(null); private set
    var loading by mutableStateOf(false); private set
    var inspecting by mutableStateOf(false); private set
    var busy by mutableStateOf(false); private set
    var error by mutableStateOf<String?>(null); private set
    var notice by mutableStateOf<String?>(null); private set
    private var listing = 0
    private var inspection = 0
    private fun active() = generation == store.state.value.generation && !store.state.value.loading
    private suspend fun call(method: String, values: JsonObject = buildJsonObject {}) =
        store.hostQuery(method, if (method == "resolveToolCall") values else JsonObject(values + ("connectorId" to JsonPrimitive(connectorId))), expectedGeneration = generation).jsonObject

    private suspend fun load(more: Boolean = false) {
        if (!active() || busy || loading || (more && next == null)) return
        val sequence = ++listing
        val visibleCount = requests.size
        loading = true
        try {
            val settings = call("getMcpToolSettings")
            var response = call("listMcpToolCalls", buildJsonObject { if (more) next?.let { put("afterId", it) } })
            if (sequence != listing || !active()) return
            policies = WireJson.decodeFromJsonElement(settings.getValue("items"))
            val rows = WireJson.decodeFromJsonElement<List<McpToolRequest>>(response.getValue("items")).toMutableList()
            var cursor = response["next"]?.jsonPrimitive?.contentOrNull
            while (!more && rows.size < visibleCount && cursor != null) {
                response = call("listMcpToolCalls", buildJsonObject { put("afterId", cursor) })
                if (sequence != listing || !active()) return
                rows += WireJson.decodeFromJsonElement<List<McpToolRequest>>(response.getValue("items"))
                cursor = response["next"]?.jsonPrimitive?.contentOrNull
            }
            requests = if (more) (requests + rows).distinctBy { it.id } else rows
            next = cursor
            detail = detail?.let { saved -> requests.firstOrNull { it.id == saved.id }?.let { saved.copy(status = it.status) } }
            error = null
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (failure: Exception) { if (sequence == listing) error = failure.message ?: "Couldn't load tools and requests." }
        finally { if (sequence == listing) loading = false }
    }
    suspend fun watch() { while (true) { load(); delay(5_000) } }
    fun refresh(more: Boolean = false) { scope.launch { load(more) } }
    fun inspect(request: McpToolRequest) {
        if (!active() || busy) return
        val sequence = ++inspection
        detail = null; inspecting = true; error = null
        scope.launch {
            try {
                val response = call("getMcpToolCall", buildJsonObject { put("id", request.id) })
                if (sequence == inspection && active()) detail = WireJson.decodeFromJsonElement(response)
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { if (sequence == inspection) error = failure.message ?: "Couldn't load exact arguments." }
            finally { if (sequence == inspection) inspecting = false }
        }
    }
    fun closeDetail() { ++inspection; detail = null; inspecting = false }
    private fun mutate(method: String, values: JsonObject, success: String) {
        if (busy || !active()) return
        ++listing; loading = false; busy = true; error = null; notice = null
        scope.launch {
            try { call(method, values); closeDetail(); notice = success }
            catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { error = failure.message ?: "Couldn't update tools and requests." }
            finally { busy = false }
            if (error == null) load()
        }
    }
    fun setPolicy(tool: String, mode: String) = mutate("setMcpToolPolicy",
        buildJsonObject { put("tool", tool); put("mode", mode) }, "Policy saved. Earlier authorizations for this tool were revoked.")
    fun resetPolicy(tool: String) = mutate("resetMcpToolPolicy",
        buildJsonObject { put("tool", tool) }, "Server default restored. Earlier authorizations were revoked.")
    fun decide(id: String, approve: Boolean) = mutate("resolveToolCall",
        buildJsonObject { put("id", id); put("approve", approve) },
        if (approve) "Decision recorded. The waiting proxy may run this request once." else "Request declined.")
    fun revokeAll() = mutate("revokeMcpToolCalls", buildJsonObject {},
        "Current requests revoked. Future requests still follow their policies.")
    fun retrieveResult(id: String) {
        if (!active()) return
        val sequence = ++inspection
        receipt = null; inspecting = true; error = null
        scope.launch {
            try {
                val response = call("getMcpToolResult", buildJsonObject { put("id", id) })
                if (sequence == inspection && active()) receipt = WireJson.decodeFromJsonElement<McpReceipt>(response).copy(id = id)
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { if (sequence == inspection) error = failure.message ?: "Couldn't retrieve the saved result." }
            finally { if (sequence == inspection) inspecting = false }
        }
    }
}
