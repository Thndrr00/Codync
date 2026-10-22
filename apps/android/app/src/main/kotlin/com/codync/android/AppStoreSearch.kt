package com.codync.android

import com.codync.android.core.*
import kotlinx.coroutines.*
import kotlinx.serialization.json.*

// Conversation search (idea 30): the host already pages chat-visible messages and returns the
// context around a hit; this store exposes both so the search sheet can jump without inventing data.

/** One hit from `searchMessages`: which message matched, its text, and the thread it lives in. */
data class SearchHit(val id: String, val botId: String, val threadId: String?, val text: String, val seq: Long, val createdAt: Long)

/** A page of hits with the cursor for the next one (`null`: no more). */
data class SearchPage(val hits: List<SearchHit>, val next: Long?)

/** Searches one bot's chat-visible messages (main chat and threads) for [query], newest first. */
suspend fun AppStore.searchMessages(botId: String, query: String, beforeSeq: Long? = null): SearchPage {
    val payload = buildJsonObject {
        put("query", query)
        put("botIds", JsonArray(listOf(JsonPrimitive(botId))))
        put("limit", 30)
        beforeSeq?.let { put("beforeSeq", it) }
    }
    val response = hostQuery("searchMessages", payload)
    val hits = response.jsonObject.getValue("results").jsonArray.map { element ->
        val o = element.jsonObject
        SearchHit(
            id = o.getValue("id").jsonPrimitive.content,
            botId = o.getValue("botId").jsonPrimitive.content,
            threadId = o["threadId"]?.takeUnless { it == JsonNull }?.jsonPrimitive?.content,
            text = o["text"]?.jsonPrimitive?.contentOrNull.orEmpty(),
            seq = o["seq"]?.jsonPrimitive?.longOrNull ?: 0L,
            createdAt = o["createdAt"]?.jsonPrimitive?.longOrNull ?: 0L,
        )
    }
    val next = response.jsonObject["next"]?.takeUnless { it == JsonNull }?.jsonPrimitive?.longOrNull
    return SearchPage(hits, next)
}

/** Loads the span around a hit into the mirror, so the jump has its message and neighbours. */
suspend fun AppStore.loadMessageContext(botId: String, entryId: String) {
    val response = hostQuery("messageContext", body("botId" to botId, "entryId" to entryId))
    val entries = response.jsonObject.getValue("entries").jsonArray.map { Entry.decode(it.jsonObject) }
    withContext(Dispatchers.IO) { requireNotNull(database).receive(entries) }
    refresh()
}

/** A search hit's jump: load the span around it, then open its bot and thread (idea 30). */
fun AppStore.openSearchHit(botId: String, entryId: String, threadId: String?) = action("search:" + entryId) {
    try { loadMessageContext(botId, entryId) } catch (_: Exception) {}
    openBot(botId, threadId)
}
