package com.codync.android

import java.io.IOException
import kotlin.coroutines.Continuation
import kotlin.coroutines.resume
import kotlin.coroutines.suspendCoroutine
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import kotlinx.coroutines.yield
import org.junit.Assert.*
import org.junit.Test

class ConversationSearchModelTest {
    @Test fun `a late reply cannot replace a newer query even when transport ignores cancellation`() = runBlocking {
        val host = SearchHost()
        val oldReply = DelayedReply<List<ChatHit>>()
        val returned = CompletableDeferred<Unit>()
        host.search = { request ->
            if (request.query == "old") {
                val hits = oldReply.await()
                returned.complete(Unit)
                hits
            } else listOf(hit("new"))
        }
        val model = ConversationSearchModel(host, "bot", this)
        try {
            model.updateQuery("old")
            withTimeout(5_000) { oldReply.requested.await() }
            model.updateQuery("new")
            awaitState(model) { it.phase == SearchPhase.Ready }
            oldReply.complete(listOf(hit("old")))
            withTimeout(5_000) { returned.await() }
            yield()
            assertEquals(listOf("new"), model.state.hits.map { it.id })
            assertNull(model.state.error)
        } finally {
            oldReply.complete(emptyList())
            model.dispose()
        }
    }

    @Test fun `a reply from a retired account or computer is ignored`() = runBlocking {
        rejectLateReply(this) { host, _ -> host.current = false }
    }

    @Test fun `a disposed screen ignores a reply that finishes after cancellation`() = runBlocking {
        rejectLateReply(this) { _, model -> model.dispose() }
    }

    @Test fun `an unknown method probe leaves All search usable and scoped search disabled`() = runBlocking {
        val host = SearchHost()
        host.saved = { throw IOException("unknown method") }
        host.search = { listOf(hit("found")) }
        val model = ConversationSearchModel(host, "bot", this)
        try {
            model.loadShortcuts()
            awaitState(model) { it.shortcutPhase == ShortcutPhase.Unavailable }
            assertTrue(model.state.shortcutError.orEmpty().contains("unknown method"))
            model.updateScope(MessageSearchScope.Main)
            model.updateQuery("release")
            awaitState(model) { it.phase == SearchPhase.Ready }
            assertEquals(MessageSearchScope.All, model.state.location)
            assertEquals(listOf("release"), host.requests.map { it.query })
            assertEquals(listOf("found"), model.state.hits.map { it.id })
        } finally { model.dispose() }
    }

    @Test fun `a failed search shows the error and retry completes it`() = runBlocking {
        val host = SearchHost()
        var attempts = 0
        host.search = {
            attempts++
            if (attempts == 1) throw IOException("Connection lost") else listOf(hit("found"))
        }
        val model = ConversationSearchModel(host, "bot", this)
        try {
            model.updateQuery("release")
            awaitState(model) { it.phase == SearchPhase.Failed }
            assertEquals("Connection lost", model.state.error)
            model.retry()
            awaitState(model) { it.phase == SearchPhase.Ready }
            assertEquals(listOf("found"), model.state.hits.map { it.id })
            assertNull(model.state.error)
        } finally { model.dispose() }
    }

    @Test fun `whose words and tool traces travel to the host`() = runBlocking {
        val host = SearchHost()
        val model = ConversationSearchModel(host, "bot", this)
        try {
            model.updateSide(ChatSearchSide.Bot)
            model.toggleTools()
            model.updateQuery("release")
            awaitState(model) { it.phase == SearchPhase.Ready }
            assertEquals(listOf("release" to (ChatSearchSide.Bot to true)), host.requests.map { it.query to (it.side to it.tools) })
        } finally { model.dispose() }
    }

    @Test fun `the scope keeps one side of the chat and leaves pages out of threads`() = runBlocking {
        val host = SearchHost()
        host.search = { listOf(pageHit("page"), hit("main"), hit("thread", threadId = "t1")) }
        val model = ConversationSearchModel(host, "bot", this)
        try {
            model.loadShortcuts()
            awaitState(model) { it.shortcutPhase == ShortcutPhase.Ready }
            model.updateQuery("release")
            awaitState(model) { it.phase == SearchPhase.Ready }
            assertEquals(listOf("page", "main", "thread"), model.state.hits.map { it.id })
            model.updateScope(MessageSearchScope.Main)
            awaitState(model) { it.hits.size == 2 }
            assertEquals(listOf("page", "main"), model.state.hits.map { it.id })
            model.updateScope(MessageSearchScope.Threads)
            awaitState(model) { it.hits.size == 1 }
            assertEquals(listOf("thread"), model.state.hits.map { it.id })
        } finally { model.dispose() }
    }

    @Test fun `a page opens in place while a message jumps into the chat`() = runBlocking {
        val host = SearchHost()
        host.search = { listOf(pageHit("doc"), hit("message")) }
        host.page = { "Title" to "Body" }
        val opened = mutableListOf<Pair<String, String?>>()
        host.open = { botId, threadId -> opened += botId to threadId }
        var closed = 0
        val model = ConversationSearchModel(host, "bot", this)
        try {
            model.updateQuery("release")
            awaitState(model) { it.phase == SearchPhase.Ready }
            model.open(model.state.hits.first { it.isPage }) { closed++ }
            awaitState(model) { it.page != null }
            assertEquals("Title" to "Body", model.state.page)
            assertEquals(0, closed)
            model.closePage()
            assertNull(model.state.page)
            model.open(model.state.hits.first { !it.isPage }) { closed++ }
            awaitState(model) { closed == 1 }
            assertEquals(listOf("bot" to null), opened)
        } finally { model.dispose() }
    }

    @Test fun `a failed save keeps loaded shortcuts and exposes the failure`() = runBlocking {
        val shortcut = SearchShortcut("saved", "release", "threads")
        val host = SearchHost()
        host.saved = { listOf(shortcut) }
        host.save = { _, _ -> throw IOException("Couldn't reach the host") }
        val model = ConversationSearchModel(host, "bot", this)
        try {
            model.loadShortcuts()
            awaitState(model) { it.shortcutPhase == ShortcutPhase.Ready }
            model.restore(shortcut)
            model.save()
            awaitState(model) { it.shortcutError != null && !it.shortcutBusy }
            assertEquals(listOf(shortcut), model.state.shortcuts)
            assertEquals("Couldn't reach the host", model.state.shortcutError)
            assertEquals(ShortcutPhase.Ready, model.state.shortcutPhase)
            assertEquals(listOf("release" to "threads"), host.saves)
        } finally { model.dispose() }
    }

    private suspend fun rejectLateReply(scope: CoroutineScope, retire: (SearchHost, ConversationSearchModel) -> Unit) {
        val host = SearchHost()
        val reply = DelayedReply<List<ChatHit>>()
        val returned = CompletableDeferred<Unit>()
        host.search = {
            val hits = reply.await()
            returned.complete(Unit)
            hits
        }
        val model = ConversationSearchModel(host, "bot", scope)
        try {
            model.updateQuery("release")
            withTimeout(5_000) { reply.requested.await() }
            retire(host, model)
            reply.complete(listOf(hit("retired")))
            withTimeout(5_000) { returned.await() }
            yield()
            assertTrue(model.state.hits.isEmpty())
            assertNull(model.state.error)
            assertNotEquals(SearchPhase.Ready, model.state.phase)
        } finally {
            reply.complete(emptyList())
            model.dispose()
        }
    }

    private suspend fun awaitState(model: ConversationSearchModel, ready: (ConversationSearchState) -> Boolean) {
        withTimeout(5_000) { while (!ready(model.state)) yield() }
    }

    /** suspendCoroutine deliberately models a response source that does not honor Job cancellation. */
    private class DelayedReply<T> {
        val requested = CompletableDeferred<Unit>()
        private var continuation: Continuation<T>? = null
        private var completed = false
        suspend fun await(): T = suspendCoroutine { continuation = it; requested.complete(Unit) }
        fun complete(value: T) {
            val waiting = continuation ?: return
            if (completed) return
            completed = true
            waiting.resume(value)
        }
    }

    private data class Request(val bot: String, val query: String, val side: ChatSearchSide, val tools: Boolean)
    private class SearchHost : ConversationSearchHost {
        var current = true
        val requests = mutableListOf<Request>()
        val saves = mutableListOf<Pair<String, String>>()
        var search: suspend (Request) -> List<ChatHit> = { emptyList() }
        var saved: suspend () -> List<SearchShortcut> = { emptyList() }
        var save: suspend (String, String) -> List<SearchShortcut> = { _, _ -> emptyList() }
        var page: suspend () -> Pair<String, String> = { "" to "" }
        var open: (String, String?) -> Unit = { _, _ -> }
        override fun isCurrent() = current
        override suspend fun savedSearches() = saved()
        override suspend fun saveSearch(query: String, searchScope: String): List<SearchShortcut> {
            saves += query to searchScope
            return save(query, searchScope)
        }
        override suspend fun deleteSavedSearch(id: String): List<SearchShortcut> = emptyList()
        override suspend fun searchChat(botId: String, query: String, side: ChatSearchSide, tools: Boolean): List<ChatHit> {
            val request = Request(botId, query, side, tools)
            requests += request
            return search(request)
        }
        override suspend fun searchPage(id: String): Pair<String, String> = page()
        override suspend fun loadMessageContext(botId: String, entryId: String) = Unit
        override fun openBot(botId: String, threadId: String?) = open(botId, threadId)
    }

    companion object {
        private fun hit(id: String, threadId: String? = null) = ChatHit("message", id, 1L, "[$id]", "agent", null, null, threadId)
        private fun pageHit(id: String) = ChatHit("document", id, 1L, "[$id]", null, null, "Title", null)
    }
}
