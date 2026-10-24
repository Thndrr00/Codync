package com.codync.android

import android.util.AtomicFile
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.codync.android.core.WireJson
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.*
import java.io.File
import java.security.MessageDigest
import java.util.UUID

@Serializable internal data class BotDocument(
    val id: String, val botId: String, val title: String, val body: String = "",
    val parentId: String? = null, val revision: Long, val updatedAt: Long = 0,
)
@Serializable internal data class DocumentDraft(
    val title: String, val body: String, val ifRevision: Long,
    val id: String? = null, val parentId: String? = null,
    val clientNonce: String? = null,
)
@Serializable internal data class DocumentReview(
    val id: String, val botId: String, val status: String, val title: String = "",
    val draft: DocumentDraft? = null, val document: BotDocument? = null,
    val createdAt: Long = 0, val expiresAt: Long = 0,
)
@Serializable internal data class BotComputerActivity(
    val id: Long, val action: String, val actorKind: String, val actorId: String,
    val result: String, val createdAt: Long,
)

/** Host owns documents and decisions; this model owns unsaved editor state. */
internal class DocumentsModel(
    private val botId: String, private val store: AppStore, private val scope: CoroutineScope,
    private val generation: Int,
) {
    var items by mutableStateOf<List<BotDocument>>(emptyList()); private set
    var reviews by mutableStateOf<List<DocumentReview>>(emptyList()); private set
    var activity by mutableStateOf<List<BotComputerActivity>>(emptyList()); private set
    var query by mutableStateOf(""); private set
    var next by mutableStateOf<String?>(null); private set
    var loading by mutableStateOf(false); private set
    var busy by mutableStateOf(false); private set
    var saving by mutableStateOf(false); private set
    var error by mutableStateOf<String?>(null); private set
    var editor by mutableStateOf(false); private set
    var draft by mutableStateOf<DocumentDraft?>(null); private set
    var dirty by mutableStateOf(false); private set
    var conflict by mutableStateOf<BotDocument?>(null); private set
    var review by mutableStateOf<DocumentReview?>(null); private set
    var reviewCurrent by mutableStateOf<BotDocument?>(null); private set
    var reviewOpen by mutableStateOf(false); private set
    private var autosave: Job? = null
    private var listing = 0
    private var loadedQuery = ""
    private var opening = 0
    private var reviewing = 0
    private val diskWrites = Mutex()
    private var persistenceVersion = 0L
    private var restored = false
    private fun active() = store.state.value.generation == generation && !store.state.value.loading
    private val persistenceScope = store.contextScope
    // Capture the account's no-backup directory and computer, never whichever account is active later.
    private val draftFile = store.state.value.computer?.id?.let { computer ->
        val hash = MessageDigest.getInstance("SHA-256").digest(botId.toByteArray()).joinToString("") { "%02x".format(it) }
        if (computer.matches(Regex("[A-Za-z0-9_-]{22}")) && store.state.value.generation == generation &&
            store.state.value.contextId == store.local.referenceId) {
            // The mirror prefix also makes forgetComputerFiles remove this computer's document drafts.
            AtomicFile(File(store.local.directory, "mirror-$computer.document-draft-$hash.json"))
        } else null
    }

    private suspend fun restoreDraft() {
        if (restored) return
        try {
            diskWrites.withLock {
                val recoveredDraft = withContext(Dispatchers.IO) {
                    draftFile?.takeIf { it.baseFile.exists() }?.let {
                        require(it.baseFile.length() <= 8 * 1024 * 1024) { "The saved document draft is too large." }
                        WireJson.decodeFromString<DocumentDraft>(it.readFully().toString(Charsets.UTF_8))
                    }
                }
                if (recoveredDraft != null && draft == null && !editor && store.state.value.generation == generation) {
                    draft = if (recoveredDraft.id == null && recoveredDraft.clientNonce == null) recoveredDraft.copy(clientNonce = UUID.randomUUID().toString()) else recoveredDraft
                    dirty = true
                }
                restored = true
            }
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (failure: Exception) { error = failure.message ?: "Couldn't restore the local draft." }
    }

    suspend fun watch() {
        restoreDraft()
        while (true) {
            if (!busy && !saving && !loading) {
                try { load(); loadReviews(); loadActivity() }
                catch (cancelled: CancellationException) { throw cancelled }
                catch (failure: Exception) { error = failure.message ?: "Couldn't refresh documents." }
            }
            delay(5_000)
        }
    }

    private fun persistDraft() {
        val saved = draft.takeIf { dirty }
        val version = ++persistenceVersion
        persistenceScope.launch {
            try {
                diskWrites.withLock {
                    if (version != persistenceVersion) return@withLock
                    withContext(Dispatchers.IO) {
                        if (store.state.value.generation != generation) return@withContext
                        val file = draftFile ?: return@withContext
                        if (saved == null) file.delete()
                        else {
                            val bytes = WireJson.encodeToString(saved).toByteArray()
                            require(bytes.size <= 8 * 1024 * 1024) { "The document draft is too large to save locally." }
                            val output = file.startWrite()
                            try { output.write(bytes); file.finishWrite(output) }
                            catch (failure: Throwable) { file.failWrite(output); throw failure }
                        }
                    }
                }
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { error = failure.message ?: "Couldn't keep this draft on your phone." }
        }
    }

    suspend fun flushDraft() {
        ++persistenceVersion
        try {
            diskWrites.withLock {
                if (!restored && !dirty) return@withLock
                val saved = draft.takeIf { dirty }
                withContext(Dispatchers.IO) {
                    val file = draftFile ?: return@withContext
                    if (saved == null) file.delete()
                    else {
                        val bytes = WireJson.encodeToString(saved).toByteArray()
                        require(bytes.size <= 8 * 1024 * 1024) { "The document draft is too large to save locally." }
                        val output = file.startWrite()
                        try { output.write(bytes); file.finishWrite(output) }
                        catch (failure: Throwable) { file.failWrite(output); throw failure }
                    }
                }
            }
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (failure: Exception) { error = failure.message ?: "Couldn't keep this draft on your phone." }
    }

    private suspend fun call(method: String, payload: JsonObject): JsonObject =
        store.hostQuery(method, payload, expectedGeneration = generation).jsonObject

    private fun task(block: suspend () -> Unit) = scope.launch {
        try { block() }
        catch (cancelled: CancellationException) { throw cancelled }
        catch (failure: Exception) { error = failure.message ?: "Couldn't reach the computer. Your draft is kept here." }
    }

    suspend fun load(more: Boolean = false) {
        val requested = ++listing
        val search = query
        val visibleCount = if (search == loadedQuery) items.size.coerceAtLeast(100) else 100
        loading = true
        try {
            val loaded = mutableListOf<BotDocument>()
            var cursor = if (more) next else null
            do {
                val response = call("listDocuments", buildJsonObject {
                    put("botId", botId); put("query", search); put("limit", 100)
                    cursor?.let { put("afterId", it) }
                })
                if (requested != listing) return
                loaded += WireJson.decodeFromJsonElement<List<BotDocument>>(response.getValue("items"))
                cursor = (response["next"] as? JsonPrimitive)?.contentOrNull
            } while (!more && loaded.size < visibleCount && cursor != null)
            items = if (more) (items + loaded).distinctBy { it.id } else loaded
            next = cursor; loadedQuery = search
            if (!dirty) error = null
        } finally { if (requested == listing) loading = false }
    }

    fun refresh() = task { load(); loadReviews(); loadActivity() }
    fun search(value: String) { query = value; task { load() } }
    fun more() = task { load(true) }

    private suspend fun loadReviews() {
        val response = call("listDocumentReviews", buildJsonObject { put("botId", botId) })
        reviews = WireJson.decodeFromJsonElement(response.getValue("items"))
    }

    private suspend fun loadActivity() {
        val response = call("listComputerActivity", buildJsonObject { put("botId", botId) })
        activity = WireJson.decodeFromJsonElement(response.getValue("items"))
    }

    fun export(copy: (String) -> Unit) = task {
        val response = call("exportComputerActivity", buildJsonObject { put("botId", botId) })
        copy(response.getValue("csv").jsonPrimitive.content)
    }

    fun create(parent: String? = null) {
        if (busy || !active()) return
        if (dirty || saving) { editor = true; return }
        ++opening; autosave?.cancel(); error = null; conflict = null
        draft = DocumentDraft("", "", 0, parentId = parent, clientNonce = UUID.randomUUID().toString()); dirty = false; editor = true
    }

    fun open(id: String) {
        if (busy || !active()) return
        if (dirty || saving) { editor = true; return }
        val requested = ++opening
        editor = true; draft = null; error = null
        task {
            val response = call("getDocument", buildJsonObject { put("id", id) })
            if (requested == opening) {
                adopt(WireJson.decodeFromJsonElement(response.getValue("document")))
            }
        }
    }

    private fun adopt(document: BotDocument) {
        draft = DocumentDraft(document.title, document.body, document.revision, document.id, document.parentId)
        dirty = false; conflict = null; error = null
        persistDraft()
    }

    fun edit(title: String? = null, body: String? = null) {
        if (busy || !active()) return
        val current = draft ?: return
        draft = current.copy(title = title ?: current.title, body = body ?: current.body)
        dirty = true; error = null
        persistDraft()
        scheduleSave()
    }

    private fun scheduleSave() {
        autosave?.cancel()
        if (conflict != null) return
        autosave = scope.launch {
            delay(800); autosave = null
            save()
        }
    }

    fun saveNow() { autosave?.cancel(); autosave = null; scope.launch { save() } }
    fun closeEditor() { saveNow(); editor = false }
    fun resume() { editor = true }
    fun discardDraft() {
        if (busy || saving || !active()) return
        ++opening; autosave?.cancel(); draft = null; dirty = false
        conflict = null; error = null; editor = false
        persistDraft()
    }
    fun saveAsCopy() {
        if (busy || saving || !active()) return
        val current = draft ?: return
        draft = current.copy(id = null, parentId = null, ifRevision = 0, clientNonce = UUID.randomUUID().toString())
        conflict = null; error = null; dirty = true
        persistDraft(); saveNow()
    }

    private suspend fun save() {
        val sent = draft ?: return
        if (!dirty || busy || saving || conflict != null || sent.title.isBlank()) return
        saving = true
        try {
            val response = call("saveDocument", buildJsonObject {
                put("botId", botId); sent.id?.let { put("id", it) }
                if (sent.id == null) {
                    sent.parentId?.let { put("parentId", it) }
                    sent.clientNonce?.let { put("clientNonce", it) }
                }
                put("title", sent.title); put("body", sent.body); put("ifRevision", sent.ifRevision)
            })
            val document = WireJson.decodeFromJsonElement<BotDocument>(response.getValue("document"))
            if (response["status"]?.jsonPrimitive?.content == "conflict") {
                conflict = document
            } else {
                val current = draft
                if (current == sent && document.title == sent.title.trim() && document.body == sent.body) adopt(document)
                else if (current != null) {
                    // A create retry may return the original saved payload for this nonce.
                    // Bind its ID, then save any newer text as a revision-checked update.
                    draft = current.copy(id = document.id, ifRevision = document.revision)
                    dirty = true
                }
                persistDraft()
                load()
            }
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (failure: Exception) { error = failure.message ?: "Couldn't save. Your draft is kept here." }
        finally { saving = false }
        if (dirty && conflict == null && error == null) scheduleSave()
    }

    fun reloadConflict() { conflict?.let(::adopt) }
    fun overwrite() {
        val current = conflict ?: return
        draft = draft?.copy(ifRevision = current.revision)
        dirty = true
        persistDraft()
        conflict = null; saveNow()
    }

    fun move(parent: String?) = task {
        val current = draft ?: return@task
        val id = current.id ?: return@task
        if (busy || saving || dirty) return@task
        busy = true
        try {
            val response = call("moveDocument", buildJsonObject {
                put("id", id); put("parentId", parent?.let(::JsonPrimitive) ?: JsonNull); put("ifRevision", current.ifRevision)
            })
            val document = WireJson.decodeFromJsonElement<BotDocument>(response.getValue("document"))
            if (response["status"]?.jsonPrimitive?.content == "conflict") conflict = document else adopt(document)
            load()
        } finally { busy = false }
    }

    fun delete(done: () -> Unit) = task {
        val current = draft ?: return@task
        val id = current.id ?: return@task
        if (busy || saving) return@task
        busy = true; autosave?.cancel()
        try {
            val response = call("deleteDocument", buildJsonObject { put("id", id); put("ifRevision", current.ifRevision) })
            if (response["status"]?.jsonPrimitive?.content == "conflict") {
                conflict = WireJson.decodeFromJsonElement(response.getValue("document"))
            } else {
                ++opening; draft = null; dirty = false; editor = false; done(); load()
                persistDraft()
            }
        } finally { busy = false }
    }

    fun inspectReview(id: String) {
        if (busy) return
        val requested = ++reviewing
        reviewOpen = true; review = null; error = null
        reviewCurrent = null
        task {
            val response = call("getDocumentReview", buildJsonObject { put("id", id) })
            if (requested == reviewing) {
                review = WireJson.decodeFromJsonElement(response.getValue("review"))
                reviewCurrent = response["currentDocument"]?.takeUnless { it is JsonNull }?.let { WireJson.decodeFromJsonElement<BotDocument>(it) }
            }
        }
    }
    fun closeReview() { ++reviewing; reviewOpen = false }
    fun decide(approve: Boolean) = task {
        val current = review ?: return@task
        if (busy || current.status != "pending") return@task
        busy = true
        try {
            // Approval supplies no draft: the host executes only its saved payload.
            val response = call("resolveAction", buildJsonObject {
                put("id", current.id); put("decision", if (approve) "approve" else "decline")
            })
            review = WireJson.decodeFromJsonElement(response.getValue("review"))
            load(); loadReviews()
        } finally { busy = false }
    }
}
