package com.codync.android

import com.codync.android.core.*
import kotlinx.serialization.json.jsonArray

/** The model's controllable host boundary; tests can hold replies past cancellation or a host switch. */
internal interface ConversationSearchHost {
    fun isCurrent(): Boolean
    suspend fun savedSearches(): List<SearchShortcut>
    suspend fun saveSearch(query: String, searchScope: String): List<SearchShortcut>
    suspend fun deleteSavedSearch(id: String): List<SearchShortcut>
    suspend fun searchMessages(botId: String, query: String, beforeSeq: Long?, searchScope: String): SearchPage
    suspend fun loadMessageContext(botId: String, entryId: String)
    fun openBot(botId: String, threadId: String?)
    /** Author and "has file" filters read the host's context; the default keeps tests simple. */
    suspend fun matchesFilters(botId: String, entryId: String, author: String, hasFile: Boolean): Boolean = true
}

/** Captures the account and computer once. Replies from a retired connection cannot update its screen. */
internal class AppStoreConversationSearchHost(private val store: AppStore) : ConversationSearchHost {
    private data class Identity(val contextId: String, val generation: Int, val computerId: String?)
    private val identity = store.state.value.let { Identity(it.contextId, it.generation, it.computer?.id) }

    override fun isCurrent(): Boolean {
        val current = store.state.value
        return current.contextId == identity.contextId && current.generation == identity.generation &&
            current.computer?.id == identity.computerId && !current.loading
    }

    override suspend fun savedSearches(): List<SearchShortcut> {
        check(isCurrent()) { "The active account or computer changed." }
        return store.savedSearches()
    }

    override suspend fun saveSearch(query: String, searchScope: String): List<SearchShortcut> {
        check(isCurrent()) { "The active account or computer changed." }
        return store.saveSearch(query, searchScope)
    }

    override suspend fun deleteSavedSearch(id: String): List<SearchShortcut> {
        check(isCurrent()) { "The active account or computer changed." }
        return store.deleteSavedSearch(id)
    }

    override suspend fun searchMessages(botId: String, query: String, beforeSeq: Long?, searchScope: String): SearchPage {
        check(isCurrent()) { "The active account or computer changed." }
        return store.searchMessages(botId, query, beforeSeq, searchScope)
    }

    override suspend fun loadMessageContext(botId: String, entryId: String) {
        check(isCurrent()) { "The active account or computer changed." }
        store.loadMessageContext(botId, entryId)
    }

    override fun openBot(botId: String, threadId: String?) {
        check(isCurrent()) { "The active account or computer changed." }
        store.openBot(botId, threadId)
    }

    override suspend fun matchesFilters(botId: String, entryId: String, author: String, hasFile: Boolean): Boolean {
        check(isCurrent()) { "The active account or computer changed." }
        val entry = store.messageContext(botId, entryId).firstOrNull { it.id == entryId } ?: return false
        if (hasFile && entry.files.isEmpty() && entry.data["attachments"]?.jsonArray?.isNotEmpty() != true) return false
        if (author.isBlank()) return true
        val name = if (entry.kind == "user") entry.person?.name ?: "You"
        else entry.data.text("author")?.let { id -> store.state.value.bots.firstOrNull { it.id == id }?.name } ?: entry.data.text("author").orEmpty()
        return name.contains(author, ignoreCase = true)
    }
}
