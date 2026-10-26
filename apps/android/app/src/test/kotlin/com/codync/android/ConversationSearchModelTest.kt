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
        val oldReply = DelayedReply<SearchPage>()
        val returned = CompletableDeferred<Unit>()
        host.search = { request ->
            if (request.query == "old") {
                val page = oldReply.await()
                returned.complete(Unit)
                page
            } else SearchPage(listOf(hit("new")), null)
        }
        val model = ConversationSearchModel(host, "bot", this)
        try {
            model.updateQuery("old")
            withTimeout(5_000) { oldReply.requested.await() }
            model.updateQuery("new")
            awaitState(model) { it.phase == SearchPhase.Ready }
            oldReply.complete(SearchPage(listOf(hit("old")), 1L))
            withTimeout(5_000) { returned.await() }
            yield()
            assertEquals(listOf("new"), model.state.hits.map { it.id })
            assertNull(model.state.next)
            assertNull(model.state.error)
        } finally {
            oldReply.complete(SearchPage(emptyList(), null))
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
        host.search = { SearchPage(listOf(hit("found")), null) }
        val model = ConversationSearchModel(host, "bot", this)
        try {
            model.loadShortcuts()
            awaitState(model) { it.shortcutPhase == ShortcutPhase.Unavailable }
            assertTrue(model.state.shortcutError.orEmpty().contains("unknown method"))
            model.updateScope(MessageSearchScope.Main)
            model.updateQuery("release")
            awaitState(model) { it.phase == SearchPhase.Ready }
            assertEquals(MessageSearchScope.All, model.state.location)
            assertEquals(listOf("all"), host.requests.map { it.location })
            assertEquals(listOf("found"), model.state.hits.map { it.id })
        } finally { model.dispose() }
    }

    @Test fun `a failed next page retains rows and cursor and retry completes the page`() = runBlocking {
        val host = SearchHost()
        var moreRequests = 0
        host.search = { request ->
            if (request.before == null) SearchPage(listOf(hit("first")), 40L)
            else {
                moreRequests++
                if (moreRequests == 1) throw IOException("Connection lost")
                SearchPage(listOf(hit("second")), null)
            }
        }
        val model = ConversationSearchModel(host, "bot", this)
        try {
            model.updateQuery("release")
            awaitState(model) { it.phase == SearchPhase.Ready }
            model.more()
            awaitState(model) { it.phase == SearchPhase.Failed }
            assertEquals(listOf("first"), model.state.hits.map { it.id })
            assertEquals(40L, model.state.next)
            assertEquals("Connection lost", model.state.error)
            model.retry()
            awaitState(model) { it.phase == SearchPhase.Ready }
            assertEquals(listOf("first", "second"), model.state.hits.map { it.id })
            assertEquals(listOf(null, 40L, 40L), host.requests.map { it.before })
            assertNull(model.state.next)
            assertNull(model.state.error)
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
        val reply = DelayedReply<SearchPage>()
        val returned = CompletableDeferred<Unit>()
        host.search = {
            val page = reply.await()
            returned.complete(Unit)
            page
        }
        val model = ConversationSearchModel(host, "bot", scope)
        try {
            model.updateQuery("release")
            withTimeout(5_000) { reply.requested.await() }
            retire(host, model)
            reply.complete(SearchPage(listOf(hit("retired")), 40L))
            withTimeout(5_000) { returned.await() }
            yield()
            assertTrue(model.state.hits.isEmpty())
            assertNull(model.state.next)
            assertNull(model.state.error)
            assertNotEquals(SearchPhase.Ready, model.state.phase)
        } finally {
            reply.complete(SearchPage(emptyList(), null))
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

    private data class Request(val bot: String, val query: String, val before: Long?, val location: String)
    private class SearchHost : ConversationSearchHost {
        var current = true
        val requests = mutableListOf<Request>()
        val saves = mutableListOf<Pair<String, String>>()
        var search: suspend (Request) -> SearchPage = { SearchPage(emptyList(), null) }
        var saved: suspend () -> List<SearchShortcut> = { emptyList() }
        var save: suspend (String, String) -> List<SearchShortcut> = { _, _ -> emptyList() }
        override fun isCurrent() = current
        override suspend fun savedSearches() = saved()
        override suspend fun saveSearch(query: String, searchScope: String): List<SearchShortcut> {
            saves += query to searchScope
            return save(query, searchScope)
        }
        override suspend fun deleteSavedSearch(id: String): List<SearchShortcut> = emptyList()
        override suspend fun searchMessages(botId: String, query: String, beforeSeq: Long?, searchScope: String): SearchPage {
            val request = Request(botId, query, beforeSeq, searchScope)
            requests += request
            return search(request)
        }
        override suspend fun loadMessageContext(botId: String, entryId: String) = Unit
        override fun openBot(botId: String, threadId: String?) = Unit
    }

    companion object {
        private fun hit(id: String) = SearchHit(id, "bot", null, id, 1L, 1L)
    }
}
