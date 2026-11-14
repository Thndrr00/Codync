package com.codync.android

import com.codync.android.core.*
import kotlinx.coroutines.*
import kotlinx.serialization.json.*

// Conversation search (idea 30): the host already knows what a chat said, which channel of it and
// what a hit's context is; this store exposes both so the search sheet can jump without inventing data.

/** Whose words a chat search keeps; everyone sends no filter. */
internal enum class ChatSearchSide(val wire: String?, val title: String) {
    Everyone(null, "Everyone"), Me("user", "Me"), Bot("agent", "Bot")
}

/** One hit inside a chat: a message with its snippet, a page of the bot, or one of its memory notes. */
data class ChatHit(
    val kind: String,
    val id: String,
    val at: Long,
    val snippet: String,
    val entryKind: String?,
    val author: String?,
    val title: String?,
    val threadId: String?,
    val shared: Boolean = false,
) {
    val isPage get() = kind == "document"
    val isNote get() = kind == "memory"
    val isMine get() = entryKind == "user"
}

/** What this chat knows about [query]: its messages, its pages and its memory, best match first. */
suspend fun AppStore.searchChat(botId: String, query: String, side: ChatSearchSide, scope: MessageSearchScope,
                                tools: Boolean, limit: Int = 30): List<ChatHit> {
    val payload = buildJsonObject {
        put("botId", botId)
        put("query", query)
        put("limit", limit)
        put("tools", tools)
        side.wire?.let { put("author", it) }
        scope.channel?.let { put("channel", it) }
    }
    val response = hostQuery("search", payload)
    return response.jsonObject.getValue("hits").jsonArray.map { element ->
        val o = element.jsonObject
        ChatHit(
            kind = o.getValue("kind").jsonPrimitive.content,
            id = o.getValue("id").jsonPrimitive.content,
            at = o["at"]?.jsonPrimitive?.longOrNull ?: 0L,
            snippet = o["snippet"]?.jsonPrimitive?.contentOrNull.orEmpty(),
            entryKind = o["entryKind"]?.takeUnless { it == JsonNull }?.jsonPrimitive?.contentOrNull,
            author = o["author"]?.takeUnless { it == JsonNull }?.jsonPrimitive?.contentOrNull,
            title = o["title"]?.takeUnless { it == JsonNull }?.jsonPrimitive?.contentOrNull,
            threadId = o["threadId"]?.takeUnless { it == JsonNull }?.jsonPrimitive?.contentOrNull,
            shared = o["shared"]?.jsonPrimitive?.booleanOrNull ?: false,
        )
    }
}

/** A page a search found: its title and body. */
suspend fun AppStore.searchPage(id: String): Pair<String, String> {
    val document = hostQuery("getDocument", body("id" to id)).jsonObject.getValue("document").jsonObject
    return document["title"]?.jsonPrimitive?.contentOrNull.orEmpty() to document["body"]?.jsonPrimitive?.contentOrNull.orEmpty()
}

/** A memory note a search found: its content. */
suspend fun AppStore.searchNote(botId: String, file: String, shared: Boolean): String {
    val response = hostQuery("memoryFile", body("botId" to botId, "file" to file, "shared" to shared))
    return response.jsonObject["content"]?.jsonPrimitive?.contentOrNull.orEmpty()
}

data class SearchShortcut(val id: String, val query: String, val scope: String)

private fun searchShortcuts(response: JsonElement): List<SearchShortcut> = response.jsonObject.getValue("searches").jsonArray.map {
    val value = it.jsonObject
    SearchShortcut(value.getValue("id").jsonPrimitive.content, value.getValue("query").jsonPrimitive.content, value.getValue("scope").jsonPrimitive.content)
}

suspend fun AppStore.savedSearches(): List<SearchShortcut> = searchShortcuts(hostQuery("savedSearches", buildJsonObject {}))
suspend fun AppStore.saveSearch(query: String, searchScope: String): List<SearchShortcut> =
    searchShortcuts(hostQuery("saveSearch", body("query" to query, "scope" to searchScope)))
suspend fun AppStore.deleteSavedSearch(id: String): List<SearchShortcut> =
    searchShortcuts(hostQuery("deleteSavedSearch", body("id" to id)))

/** Loads the span around a hit into the mirror, so the jump has its message and neighbours. */
suspend fun AppStore.loadMessageContext(botId: String, entryId: String) {
    val response = hostQuery("messageContext", body("botId" to botId, "entryId" to entryId))
    val entries = response.jsonObject.getValue("entries").jsonArray.map { Entry.decode(it.jsonObject) }
    withContext(Dispatchers.IO) { requireNotNull(database).receive(entries) }
    refresh()
}

