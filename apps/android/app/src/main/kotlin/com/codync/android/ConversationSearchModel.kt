package com.codync.android

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

internal enum class MessageSearchScope(val wire: String, val title: String) {
    All("all", "All"), Main("main", "Main chat"), Threads("threads", "Threads")
}
internal enum class SearchPhase { Idle, Searching, LoadingMore, Ready, Failed }
internal enum class ShortcutPhase { Loading, Ready, Unavailable }

internal data class ConversationSearchState(
    val query: String = "",
    val location: MessageSearchScope = MessageSearchScope.All,
    val hits: List<SearchHit> = emptyList(),
    val next: Long? = null,
    val phase: SearchPhase = SearchPhase.Idle,
    val error: String? = null,
    val shortcuts: List<SearchShortcut> = emptyList(),
    val shortcutPhase: ShortcutPhase = ShortcutPhase.Loading,
    val shortcutError: String? = null,
    val shortcutBusy: Boolean = false,
    val showShortcuts: Boolean = true,
    val opening: String? = null,
)

/** The host owns saved searches and filtering. This model owns request and screen lifetimes. */
internal class ConversationSearchModel(
    private val host: ConversationSearchHost,
    private val botId: String,
    parentScope: CoroutineScope,
) {
    var state by mutableStateOf(ConversationSearchState()); private set
    private val lifetime = SupervisorJob(parentScope.coroutineContext[Job])
    private val scope = CoroutineScope(parentScope.coroutineContext + lifetime)
    private var searchJob: Job? = null
    private var shortcutsJob: Job? = null
    private var openingJob: Job? = null
    private var sequence = 0
    private var shortcutSequence = 0
    private var disposed = false

    private fun active() = !disposed && lifetime.isActive && host.isCurrent()
    private fun current(request: Int) = active() && request == sequence

    fun dispose() {
        disposed = true
        sequence++
        shortcutSequence++
        scope.cancel()
    }

    fun updateQuery(query: String) {
        if (!active() || query == state.query) return
        state = state.copy(query = query, showShortcuts = query.isBlank())
        search()
    }

    fun updateScope(location: MessageSearchScope) {
        if (!active() || state.shortcutPhase != ShortcutPhase.Ready || location == state.location) return
        state = state.copy(location = location)
        search()
    }

    fun restore(shortcut: SearchShortcut) {
        if (!active() || state.shortcutPhase != ShortcutPhase.Ready) return
        val location = MessageSearchScope.entries.firstOrNull { it.wire == shortcut.scope }
        if (location == null) {
            state = state.copy(shortcutError = "This saved search uses a scope this app does not support.")
            return
        }
        state = state.copy(query = shortcut.query, location = location, shortcutError = null, showShortcuts = false)
        search()
    }

    /** A failed capability probe disables only saving and scoped search, never ordinary All search. */
    fun loadShortcuts() {
        if (!active() || state.shortcutBusy) return
        shortcutsJob?.cancel()
        val request = ++shortcutSequence
        state = state.copy(shortcutPhase = ShortcutPhase.Loading, shortcutError = null)
        shortcutsJob = scope.launch {
            try {
                val shortcuts = host.savedSearches()
                if (active() && request == shortcutSequence) {
                    state = state.copy(shortcuts = shortcuts, shortcutPhase = ShortcutPhase.Ready)
                }
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) {
                if (active() && request == shortcutSequence) {
                    val resetScope = state.location != MessageSearchScope.All
                    state = state.copy(shortcutPhase = ShortcutPhase.Unavailable, location = MessageSearchScope.All,
                        shortcutError = "Saved searches and scope filters are unavailable: ${error.message ?: "Request failed."}")
                    if (resetScope) search()
                }
            }
        }
    }

    fun toggleShortcuts() { state = state.copy(showShortcuts = !state.showShortcuts) }

    fun save() {
        if (!active() || state.shortcutPhase != ShortcutPhase.Ready || state.shortcutBusy || state.query.isBlank()) return
        val query = state.query.trim()
        val location = state.location.wire
        mutateShortcuts { host.saveSearch(query, location) }
    }

    fun remove(shortcut: SearchShortcut) {
        if (!active() || state.shortcutPhase != ShortcutPhase.Ready || state.shortcutBusy) return
        mutateShortcuts { host.deleteSavedSearch(shortcut.id) }
    }

    private fun mutateShortcuts(operation: suspend () -> List<SearchShortcut>) {
        val request = ++shortcutSequence
        shortcutsJob?.cancel()
        state = state.copy(shortcutBusy = true, shortcutError = null)
        shortcutsJob = scope.launch {
            try {
                val shortcuts = operation()
                if (active() && request == shortcutSequence) state = state.copy(shortcuts = shortcuts)
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) {
                if (active() && request == shortcutSequence) state = state.copy(shortcutError = error.message ?: "Couldn't update saved searches.")
            } finally {
                if (active() && request == shortcutSequence) state = state.copy(shortcutBusy = false)
            }
        }
    }

    fun retry() { search(more = state.hits.isNotEmpty() && state.next != null) }
    fun more() { if (state.next != null && state.phase !in listOf(SearchPhase.Searching, SearchPhase.LoadingMore)) search(more = true) }

    private fun search(more: Boolean = false) {
        searchJob?.cancel()
        openingJob?.cancel()
        val request = ++sequence
        val query = state.query.trim()
        val location = state.location
        val before = if (more) state.next else null
        val previous = if (more) state.hits else emptyList()
        state = state.copy(hits = previous, next = before, error = null, opening = null,
            phase = if (query.isEmpty()) SearchPhase.Idle else if (more) SearchPhase.LoadingMore else SearchPhase.Searching)
        if (query.isEmpty() || !active()) return
        // Main/Threads are offered only after the host has answered the feature probe.
        if (location != MessageSearchScope.All && state.shortcutPhase != ShortcutPhase.Ready) {
            state = state.copy(hits = emptyList(), next = null, phase = SearchPhase.Failed, error = "Scope filters are unavailable. Select All or retry saved searches.")
            return
        }
        searchJob = scope.launch {
            try {
                if (!more) delay(300)
                if (!current(request)) return@launch
                val page = host.searchMessages(botId, query, before, location.wire)
                if (current(request)) state = state.copy(hits = previous + page.hits, next = page.next, phase = SearchPhase.Ready)
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) {
                if (current(request)) state = state.copy(phase = SearchPhase.Failed, error = error.message ?: "Search failed.")
            }
        }
    }

    fun open(hit: SearchHit, close: () -> Unit) {
        if (!active() || state.opening != null || hit.botId != botId) return
        val request = sequence
        state = state.copy(opening = hit.id, error = null)
        openingJob = scope.launch {
            try {
                host.loadMessageContext(botId, hit.id)
                if (current(request)) {
                    host.openBot(botId, hit.threadId)
                    close()
                }
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) {
                if (current(request)) state = state.copy(error = error.message ?: "Couldn't open this search result.")
            } finally {
                if (current(request)) state = state.copy(opening = null)
            }
        }
    }
}
