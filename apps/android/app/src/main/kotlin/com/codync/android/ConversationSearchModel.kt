package com.codync.android

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import java.util.Calendar
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

/** The date filter; `since` is the host channel-history bound computed from the phone's clock. */
internal enum class SearchDateRange(val title: String, val days: Int?) {
    Any("Any time", null), Today("Today", 0), Week("Past week", 7), Month("Past month", 30);

    fun since(): Long {
        if (days == null) return 0L
        val calendar = Calendar.getInstance()
        if (days == 0) {
            calendar.set(Calendar.HOUR_OF_DAY, 0)
            calendar.set(Calendar.MINUTE, 0)
            calendar.set(Calendar.SECOND, 0)
            calendar.set(Calendar.MILLISECOND, 0)
        } else calendar.add(Calendar.DAY_OF_YEAR, -days!!)
        return calendar.timeInMillis
    }
}

internal enum class SearchPhase { Idle, Searching, LoadingMore, Ready, Failed }
internal enum class ShortcutPhase { Loading, Ready, Unavailable }

internal data class ConversationSearchState(
    val query: String = "",
    val location: MessageSearchScope = MessageSearchScope.All,
    val dateRange: SearchDateRange = SearchDateRange.Any,
    val author: String = "",
    val hasFile: Boolean = false,
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
    /** The host call's start, not the keystroke: the debounce is not "Searching". */
    val searchStartedAt: Long? = null,
    val showSpinner: Boolean = false,
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
    private var spinnerJob: Job? = null
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
        val cleared = state.query.isBlank() && query.isNotBlank()
        state = state.copy(query = query, showShortcuts = query.isBlank())
        search(queryChanged = true, cleared = cleared)
    }

    fun updateScope(location: MessageSearchScope) {
        if (!active() || state.shortcutPhase != ShortcutPhase.Ready || location == state.location) return
        state = state.copy(location = location)
        search()
    }

    fun updateDateRange(range: SearchDateRange) {
        if (!active() || range == state.dateRange) return
        state = state.copy(dateRange = range)
        search()
    }

    fun updateAuthor(author: String) {
        if (!active() || author == state.author) return
        state = state.copy(author = author)
        search()
    }

    fun updateHasFile(hasFile: Boolean) {
        if (!active() || hasFile == state.hasFile) return
        state = state.copy(hasFile = hasFile)
        search()
    }

    fun restore(shortcut: SearchShortcut) {
        if (!active() || state.shortcutPhase != ShortcutPhase.Ready) return
        val location = MessageSearchScope.entries.firstOrNull { it.wire == shortcut.scope }
        if (location == null) {
            state = state.copy(shortcutError = "This saved search uses a scope this app does not support.")
            return
        }
        val cleared = state.query.isBlank()
        state = state.copy(query = shortcut.query, location = location, shortcutError = null, showShortcuts = false)
        search(queryChanged = true, cleared = cleared)
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

    private fun search(more: Boolean = false, queryChanged: Boolean = false, cleared: Boolean = false) {
        searchJob?.cancel()
        spinnerJob?.cancel()
        openingJob?.cancel()
        val request = ++sequence
        val query = state.query.trim()
        val location = state.location
        val before = if (more) state.next else null
        val previous = if (more) state.hits
        else if (query.isEmpty() || !hitsAfter(queryChanged, cleared)) emptyList()
        else state.hits
        state = state.copy(hits = previous, next = before, error = null, opening = null,
            searchStartedAt = null, showSpinner = false,
            phase = if (query.isEmpty()) SearchPhase.Idle else if (more) SearchPhase.LoadingMore else SearchPhase.Searching)
        if (query.isEmpty() || !active()) return
        // Main/Threads are offered only after the host has answered the feature probe.
        if (location != MessageSearchScope.All && state.shortcutPhase != ShortcutPhase.Ready) {
            state = state.copy(hits = emptyList(), next = null, phase = SearchPhase.Failed, error = "Scope filters are unavailable. Select All or retry saved searches.")
            return
        }
        searchJob = scope.launch {
            var spinner: Job? = null
            try {
                if (!more) delay(300)
                if (!current(request)) return@launch
                // The debounce isn't Searching: the spinner clock starts with the host call.
                val startedAt = System.currentTimeMillis()
                state = state.copy(searchStartedAt = startedAt)
                spinner = scope.launch {
                    delay(200)
                    if (current(request)) state = state.copy(showSpinner = showSpinner(startedAt, System.currentTimeMillis()))
                }
                spinnerJob = spinner
                val page = host.searchMessages(botId, query, before, location.wire)
                // Date is in the hit the host already returned; author and "has file" need the
                // message itself, read from the same host context the jump uses.
                val since = state.dateRange.since()
                var matched = page.hits.filter { it.createdAt >= since }
                if (state.author.isNotBlank() || state.hasFile) {
                    val author = state.author.trim()
                    val hasFile = state.hasFile
                    matched = matched.filter { hit -> host.matchesFilters(botId, hit.id, author, hasFile) }
                }
                if (current(request)) state = state.copy(hits = previous + matched, next = page.next, phase = SearchPhase.Ready,
                    showSpinner = false, searchStartedAt = null)
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) {
                if (current(request)) state = state.copy(phase = SearchPhase.Failed, error = error.message ?: "Search failed.",
                    showSpinner = false, searchStartedAt = null)
            } finally {
                spinner?.cancel()
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
