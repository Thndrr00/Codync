package com.codync.android

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.codync.android.core.WireJson
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.decodeFromJsonElement
import kotlinx.serialization.json.put

/**
 * What a bot's documentation index holds (`docsStatus`): the folder the host walks, the files it found there and
 * whether a limit cut the walk short.
 */
@Serializable internal data class DocsIndexStatus(
    val folder: String,
    val files: Int = 0,
    val bytes: Long = 0,
    val truncated: Boolean = false,
)

/**
 * One bot's documentation and code index on the computer (the `codync-docs` server): reads what the host indexed and
 * names the folder it reads. The host builds, caches and limits the index; this only shows it and points it elsewhere.
 */
internal class DocsIndexModel(private val botId: String, private val store: AppStore, private val scope: CoroutineScope) {
    var status by mutableStateOf<DocsIndexStatus?>(null); private set
    var busy by mutableStateOf(false); private set
    var error by mutableStateOf<String?>(null); private set
    private var request = 0

    // The first read walks the folder on the computer, which can take a while on a large repository.
    private suspend fun read(method: String, path: String? = null): DocsIndexStatus {
        val response = store.hostQuery(method, buildJsonObject {
            put("botId", botId)
            if (path != null) put("path", path)
        }, timeoutMillis = 60_000)
        return WireJson.decodeFromJsonElement<DocsIndexStatus>(response)
    }

    /** The index as it is now: what this bot's folder holds today, and the host's error when it can't say. */
    suspend fun load() {
        val asked = ++request
        try {
            val value = read("docsStatus")
            if (asked == request) { status = value; error = null }
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (failure: Exception) {
            if (asked == request) error = failure.message ?: "Couldn't read the documentation index."
        }
    }

    /** Names the folder to index, or with an empty [path] the bot's working folder, and shows what the host indexed. */
    fun choose(path: String) {
        if (busy) return
        val asked = ++request
        busy = true; error = null
        scope.launch {
            try {
                val value = read("setDocsFolder", path)
                if (asked == request) status = value
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { if (asked == request) error = failure.message ?: "Couldn't change the indexed folder." }
            finally { busy = false }
        }
    }
}
