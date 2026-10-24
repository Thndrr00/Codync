package com.codync.android

import androidx.compose.runtime.*
import com.codync.android.core.Entry
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import kotlinx.serialization.json.*

internal class VoiceDecisionsModel(private val store: AppStore, private val receipt: Entry, private val scope: CoroutineScope) {
    private val generation = store.state.value.generation
    private var lifetime = 0
    var summary by mutableStateOf("")
    var confirmed by mutableStateOf<String?>(null); private set
    var busy by mutableStateOf(false); private set
    var loading by mutableStateOf(true); private set
    var error by mutableStateOf<String?>(null); private set
    private fun active(token: Int) = token == lifetime && store.state.value.generation == generation && !store.state.value.loading
    fun dispose() { lifetime++ }
    fun load() {
        if (busy) return
        val token = ++lifetime
        loading = true; error = null
        scope.launch {
            try {
                val result = call("voiceDecisions")
                if (active(token)) confirmed = ((result["entry"] as? JsonObject)?.get("data") as? JsonObject)
                    ?.get("voiceDecisions")?.jsonObject?.get("summary")?.jsonPrimitive?.contentOrNull
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { if (active(token)) error = failure.message ?: "Couldn't read confirmed decisions." }
            finally { if (active(token)) loading = false }
        }
    }
    fun confirm() {
        if (busy || loading || confirmed != null || summary.isBlank() || summary.length > 4000) return
        val token = lifetime; val text = summary.trim()
        busy = true; error = null
        scope.launch {
            try { call("confirmVoiceDecisions", text); if (active(token)) confirmed = text }
            catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { if (active(token)) error = failure.message ?: "Couldn't confirm decisions." }
            finally { if (active(token)) busy = false }
        }
    }
    private suspend fun call(method: String, summary: String? = null) = store.hostQuery(method, buildJsonObject {
        put("botId", receipt.botId); put("receiptId", receipt.id); if (summary != null) put("summary", summary)
    }, expectedGeneration = generation).jsonObject
}
