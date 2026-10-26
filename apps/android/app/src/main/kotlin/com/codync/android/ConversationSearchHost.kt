package com.codync.android

/** The model's controllable host boundary; tests can hold replies past cancellation or a host switch. */
internal interface ConversationSearchHost {
    fun isCurrent(): Boolean
    suspend fun savedSearches(): List<SearchShortcut>
    suspend fun saveSearch(query: String, searchScope: String): List<SearchShortcut>
    suspend fun deleteSavedSearch(id: String): List<SearchShortcut>
    suspend fun searchMessages(botId: String, query: String, beforeSeq: Long?, searchScope: String): SearchPage
    suspend fun loadMessageContext(botId: String, entryId: String)
    fun openBot(botId: String, threadId: String?)
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
}
