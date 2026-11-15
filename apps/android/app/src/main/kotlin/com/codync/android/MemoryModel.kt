package com.codync.android

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.codync.android.core.Brain
import com.codync.android.core.BrainBundle
import com.codync.android.core.BrainCalls
import com.codync.android.core.BrainImportReply
import com.codync.android.core.BrainsReply
import com.codync.android.core.BotMemory
import com.codync.android.core.MemoryCalls
import com.codync.android.core.MemoryFileDetail
import com.codync.android.core.WireJson
import java.io.File
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.serialization.SerializationException
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.decodeFromJsonElement
import kotlinx.serialization.json.encodeToJsonElement

/** One bot's memory on the computer (iOS `MemoryStore`): fetches it, and changes it through the host. */
internal class MemoryModel(private val botId: String, private val store: AppStore, private val scope: CoroutineScope) {
    var view by mutableStateOf<BotMemory?>(null); private set
    /** Every brain with the bots in it; null until read. */
    var brains by mutableStateOf<List<Brain>?>(null); private set
    var busy by mutableStateOf(false); private set
    var error by mutableStateOf<String?>(null)
    private var request = 0

    suspend fun load() {
        val asked = ++request
        try {
            val value = WireJson.decodeFromJsonElement<BotMemory>(store.hostCall("memory", MemoryCalls.read(botId)))
            if (asked == request) { view = value; error = null }
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (failure: Exception) {
            // A host before 2.18.0 answers with its old memory listing, which has no layers or dreams.
            if (asked == request) error = if (failure is SerializationException || failure is IllegalArgumentException)
                "Update Codync on your computer to see this bot's memory." else failure.message ?: "Couldn't load memory."
        }
    }

    fun refresh() = scope.launch { load() }

    /** Every brain, and the bot's memory after the change that opened this. */
    fun loadBrains() = scope.launch {
        try {
            brains = WireJson.decodeFromJsonElement<BrainsReply>(store.hostCall("brains", BrainCalls.list())).brains
            error = null
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (failure: Exception) { error = failure.message ?: "Couldn't load the brains." }
    }

    /** Makes a brain; with `id`, renames that one. */
    fun saveBrain(name: String, id: String? = null) = change(BrainCalls.saveMethod(id), BrainCalls.name(id, name))

    /** Moves the bot into a brain (the Fleet one is the shared memory). */
    fun joinBrain(brain: Brain) = change("joinBrain", BrainCalls.join(botId, brain.id))

    /** Takes the bot out of its brain; it keeps only its own memory (picking one again brings sharing back). */
    fun leaveBrain() = change("leaveBrain", BrainCalls.bot(botId))

    /** Deletes a brain; its bots go back to the Fleet brain and what it held is gone. */
    fun deleteBrain(brain: Brain) = change("deleteBrain", BrainCalls.id(brain.id))

    /** Saves a brain as a file and opens the share sheet: another copy of Codync can import it. */
    fun exportBrain(brain: Brain) {
        if (busy) return
        busy = true; error = null
        scope.launch {
            try {
                val bundle = store.hostCall("exportBrain", BrainCalls.id(brain.id))
                val folder = withContext(Dispatchers.IO) { store.attachments.shareFolder().apply { mkdirs() } }
                val file = File(folder, "${brain.name}.brain.json")
                withContext(Dispatchers.IO) { file.writeText(WireJson.encodeToString(JsonElement.serializer(), bundle)) }
                store.attachments.open(file, file.name)
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { error = failure.message ?: "Couldn't save the brain." }
            finally { busy = false }
        }
    }

    /** A brain made from a file this app saved; answers the files the checks turned away. */
    suspend fun importBrain(bundle: BrainBundle, name: String? = null): Int? = try {
        val reply = WireJson.decodeFromJsonElement<BrainImportReply>(
            store.hostCall("importBrain", BrainCalls.import(WireJson.encodeToJsonElement(bundle), name)),
        )
        brains = reply.brains
        load()
        reply.skipped.size
    } catch (cancelled: CancellationException) { throw cancelled }
    catch (failure: Exception) { error = failure.message ?: "Couldn't import the brain."; null }

    /** A brain method that answers the brains; reloads after it. */
    private fun change(method: String, body: JsonObject) {
        if (busy) return
        busy = true; error = null
        scope.launch {
            try {
                brains = WireJson.decodeFromJsonElement<BrainsReply>(store.hostCall(method, body)).brains
                load()
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { error = failure.message ?: "Something went wrong." }
            finally { busy = false }
        }
    }

    /** Drops the error of an action that failed once the sheet has read the latest; what the user wrote stays. */
    fun dismissError() { error = null }

    /** A long-form note as it is now, with its versions. */
    suspend fun file(name: String, shared: Boolean): MemoryFileDetail? = try {
        WireJson.decodeFromJsonElement<MemoryFileDetail>(store.hostCall("memoryFile", MemoryCalls.fileRead(botId, name, shared)))
    } catch (cancelled: CancellationException) { throw cancelled }
    catch (failure: Exception) { error = failure.message ?: "Couldn't load the note."; null }

    /** Saves, deletes (`content` and `version` null) or brings back a version of a note over what was loaded (`base`); reloads after it. */
    suspend fun changeFile(name: String, shared: Boolean, base: String?, content: String? = null, version: Long? = null): Boolean {
        if (busy) return false
        busy = true; error = null
        return try {
            store.hostCall(MemoryCalls.fileMethod(content, version), MemoryCalls.fileChange(botId, name, shared, base, content, version))
            load()
            true
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (failure: Exception) { error = failure.message ?: "Something went wrong."; false }
        finally { busy = false }
    }

    /** A memory method for this bot; reloads after it. */
    fun mutate(method: String, id: Long? = null, pinned: Boolean? = null, archived: Boolean? = null, shared: Boolean? = null,
        text: String? = null, done: () -> Unit = {}) {
        if (busy) return
        busy = true; error = null
        scope.launch {
            try {
                store.hostCall(method, MemoryCalls.mutation(botId, id, pinned, archived, shared, text))
                load()
                done()
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { error = failure.message ?: "Something went wrong." }
            finally { busy = false }
        }
    }
}
