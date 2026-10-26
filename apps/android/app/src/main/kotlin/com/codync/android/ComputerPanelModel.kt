package com.codync.android

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.codync.android.core.ComputerPermissions
import com.codync.android.core.WireJson
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.*

@Serializable internal data class ComputerPanelSnapshot(
    val enabled: Boolean, val permissions: ComputerPermissions, val workspace: String,
    val screen: JsonObject, val activity: List<BotComputerActivity>, val services: JsonObject = buildJsonObject {},
)
@Serializable internal data class ComputerFileEntry(val name: String, val directory: Boolean, val link: Boolean)

internal class ComputerPanelModel(private val botId: String, private val store: AppStore,
    private val scope: CoroutineScope, private val generation: Int) {
    var snapshot by mutableStateOf<ComputerPanelSnapshot?>(null); private set
    var files by mutableStateOf<List<ComputerFileEntry>>(emptyList()); private set
    var path by mutableStateOf(""); private set
    var text by mutableStateOf<String?>(null); private set
    var truncated by mutableStateOf(false); private set
    var loading by mutableStateOf(false); private set
    var busy by mutableStateOf(false); private set
    var error by mutableStateOf<String?>(null); private set
    private var browsing = 0
    private var listing = 0
    private fun active() = store.state.value.generation == generation && !store.state.value.loading
    private suspend fun call(method: String, values: JsonObject = buildJsonObject {}) =
        store.hostQuery(method, JsonObject(values + ("botId" to JsonPrimitive(botId))), expectedGeneration = generation).jsonObject
    private suspend fun load(clearError: Boolean = false) {
        if (!active() || loading || busy) return
        val sequence = ++listing
        loading = true
        if (clearError) error = null
        try {
            val response = call("computerPanel")
            if (!active() || sequence != listing) return
            val saved = WireJson.decodeFromJsonElement<ComputerPanelSnapshot>(response)
            snapshot = saved
            if (!saved.enabled || !saved.permissions.files) { ++browsing; files = emptyList(); text = null }
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (failure: Exception) { if (sequence == listing) error = failure.message ?: "Couldn't load the computer." }
        finally { if (sequence == listing) loading = false }
    }
    suspend fun watch() { while (true) { load(); delay(5_000) } }
    fun refresh() { scope.launch { load(true) } }
    fun permissions(enabled: Boolean, permissions: ComputerPermissions, saved: () -> Unit) {
        if (!active() || busy) return
        ++listing; loading = false
        busy = true; error = null
        scope.launch {
            try {
                store.hostQuery("updateBot", buildJsonObject { put("id", botId); put("computer", enabled); put("computerPermissions", WireJson.encodeToJsonElement(permissions)) }, expectedGeneration = generation)
                if (active()) {
                    snapshot = snapshot?.copy(enabled = enabled, permissions = permissions)
                    if (!enabled || !permissions.files) { ++browsing; files = emptyList(); text = null }
                    saved()
                }
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { error = failure.message ?: "Couldn't change computer permissions." }
            finally { busy = false }
            load()
        }
    }
    fun open(path: String, directory: Boolean = true) {
        if (!active()) return
        val sequence = ++browsing
        text = null; error = null
        scope.launch {
            try {
                val response = call(if (directory) "listComputerFiles" else "readComputerFile", buildJsonObject { put("path", path) })
                if (!active() || sequence != browsing) return@launch
                if (directory) {
                    files = WireJson.decodeFromJsonElement(response.getValue("items")); this@ComputerPanelModel.path = path
                    truncated = response["truncated"]?.jsonPrimitive?.booleanOrNull == true
                } else text = response.getValue("text").jsonPrimitive.content
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { if (sequence == browsing) error = failure.message ?: "Couldn't open the workspace path." }
        }
    }
    fun protectedEntry(site: String?, id: String? = null) {
        if (!active() || busy) return
        ++listing; loading = false
        busy = true; error = null
        scope.launch {
            try {
                val values = buildJsonObject { if (site != null) { put("botId", botId); put("site", site) } else put("id", id) }
                store.hostQuery(if (site != null) "beginProtectedEntry" else "finishProtectedEntry", values, expectedGeneration = generation)
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { error = failure.message ?: "Couldn't change manual entry mode." }
            finally { busy = false }
            load()
        }
    }
}
