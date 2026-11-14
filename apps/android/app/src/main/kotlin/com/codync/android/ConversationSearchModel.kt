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

/** Where the words are looked for: everywhere, the chat itself, or its threads. `wire` also names a saved
 * search; everywhere sends no channel. */
internal enum class MessageSearchScope(val wire: String, val title: String) {
    All("all", "Everything"), Main("main", "This chat"), Threads("threads", "Threads")
}
internal enum class SearchPhase { Idle, Searching, Ready, Failed }
internal enum class ShortcutPhase { Loading, Ready, Unavailable }

/** A page or a note opened over the results. */
internal data class SearchPage(val title: String, val body: String, val note: Boolean)

internal data class ConversationSearchState(
    val query: String = "",
    val side: ChatSearchSide = ChatSearchSide.Everyone,
    val tools: Boolean = false,
    val location: MessageSearchScope = MessageSearchScope.All,
    val hits: List<ChatHit> = emptyList(),
    val phase: SearchPhase = SearchPhase.Idle,
    val error: String? = null,
    val page: SearchPage? = null,
    val shortcuts: List<SearchShortcut> = emptyList(),
    val shortcutPhase: ShortcutPhase = ShortcutPhase.Loading,
    val shortcutError: String? = null,
    val shortcutBusy: Boolean = false,
    val showShortcuts: Boolean = true,
    val opening: String? = null,
)

/** The host owns saved searches and searching. This model owns request and screen lifetimes. */
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
        if (!active() || location == state.location) return
        state = state.copy(location = location)
        search()
    }

    /** Whose words to keep: everyone, only the person's, or only the bot's. */
    fun updateSide(side: ChatSearchSide) {
        if (!active() || side == state.side) return
        state = state.copy(side = side)
        search()
    }

    /** Tool traces, thoughts and notices too, for what a tool printed. */
    fun toggleTools() {
        if (!active()) return
        state = state.copy(tools = !state.tools)
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

    /** A failed capability probe disables only saving, restoring and listing saved searches, never searching. */
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
                    state = state.copy(shortcutPhase = ShortcutPhase.Unavailable,
                        shortcutError = "Saved searches are unavailable: ${error.message ?: "Request failed."}")
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

    fun retry() { search() }

    private fun search() {
        searchJob?.cancel()
        openingJob?.cancel()
        val request = ++sequence
        val query = state.query.trim()
        val side = state.side
        val scope = state.location
        val tools = state.tools
        state = state.copy(hits = emptyList(), error = null, opening = null, page = null,
            phase = if (query.isEmpty()) SearchPhase.Idle else SearchPhase.Searching)
        if (query.isEmpty() || !active()) return
        searchJob = scope.launch {
            try {
                delay(300)
                if (!current(request)) return@launch
                val hits = host.searchChat(botId, query, side, scope, tools)
                if (current(request)) state = state.copy(hits = hits, phase = SearchPhase.Ready)
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) {
                if (current(request)) state = state.copy(phase = SearchPhase.Failed, error = error.message ?: "Search failed.")
            }
        }
    }

    fun open(hit: ChatHit, close: () -> Unit) {
        if (!active() || state.opening != null) return
        if (hit.isPage) { openPage(hit); return }
        if (hit.isNote) { openNote(hit); return }
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

    /** A page opens in place, before its library. */
    private fun openPage(hit: ChatHit) {
        val request = sequence
        state = state.copy(opening = hit.id, error = null)
        openingJob = scope.launch {
            try {
                val (title, body) = host.searchPage(hit.id)
                if (current(request)) state = state.copy(page = SearchPage(title, body, note = false))
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) {
                if (current(request)) state = state.copy(error = error.message ?: "Couldn't open this page.")
            } finally {
                if (current(request)) state = state.copy(opening = null)
            }
        }
    }

    /** A memory note opens the same way, before the memory screen. */
    private fun openNote(hit: ChatHit) {
        val request = sequence
        state = state.copy(opening = hit.id, error = null)
        openingJob = scope.launch {
            try {
                val body = host.searchNote(botId, hit.title.orEmpty(), hit.shared)
                if (current(request)) state = state.copy(page = SearchPage(hit.title.orEmpty(), body, note = true))
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) {
                if (current(request)) state = state.copy(error = error.message ?: "Couldn't open this note.")
            } finally {
                if (current(request)) state = state.copy(opening = null)
            }
        }
    }

    fun closePage() { state = state.copy(page = null) }
}
