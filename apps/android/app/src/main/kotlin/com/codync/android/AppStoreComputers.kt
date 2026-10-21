package com.codync.android

import androidx.lifecycle.viewModelScope
import com.codync.android.core.Computer
import com.codync.android.core.HostConnector
import com.codync.android.core.LinkState
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull

// The computers paired with this phone. Each keeps its own mirror and drafts; the first one in the
// saved list is the one the roster, chats and widgets show and the only one connected. Switching
// closes its link and mirror and opens the other's, keeping every computer's push registration.

/** The computer the app is showing (the first saved one), or none. */
internal val AppState.computer: Computer? get() = computers.firstOrNull()

/**
 * The one way the saved computers change: read, change and write the file under a lock, then show
 * that same list, so a hello's save and a removal can't undo each other.
 */
private suspend fun AppStore.changeComputers(change: (List<Computer>) -> List<Computer>): List<Computer> = computerWrites.withLock {
    val updated = withContext(Dispatchers.IO) { change(local.computers()).also(local::saveComputers) }
    mutableState.update { it.copy(computers = updated) }
    updated
}

/** Saves a new copy of a computer still on this phone (a removed one stays removed), in its place. */
internal suspend fun AppStore.saveComputer(computer: Computer) {
    changeComputers { saved -> saved.map { if (it.id == computer.id) computer else it } }
}

/** Shows another paired computer: its bots, chats and widgets, through its own link. */
fun AppStore.switchComputer(id: String) {
    if (state.value.computer?.id == id || state.value.computers.none { it.id == id }) return
    showComputer { computers -> computers.firstOrNull { it.id == id } }
}

/**
 * Puts `pick` (from the saved computers) first and reloads the account around it, after any change
 * already under way. The account's push registrations stay, so the computers not shown still notify
 * this phone. Nothing is torn down when `pick` finds nothing (it was removed meanwhile).
 */
internal fun AppStore.showComputer(after: suspend () -> Unit = {}, pick: (List<Computer>) -> Computer?) = sessionAction(queue = true) {
    transitions.withLock {
        val chosen = pick(withContext(Dispatchers.IO) { local.computers() }) ?: return@withLock
        if (chosen.id == state.value.computer?.id) return@withLock
        val userId = state.value.account.userId
        retireContext(Retirement.Switch)
        changeComputers { saved -> saved.filter { it.id == chosen.id } + saved.filter { it.id != chosen.id } }
        loadContext(userId)
    }
    after()
}

/** Saves a computer that was just paired or approved (kept even if the switch has to wait), and shows it. */
internal suspend fun AppStore.addComputer(computer: Computer) {
    val first = state.value.computer == null
    changeComputers { saved -> if (saved.any { it.id == computer.id }) saved.map { if (it.id == computer.id) computer else it } else saved + computer }
    // The phone's first computer is already the one shown: there's nothing to switch from, just connect.
    if (first) {
        if (database == null) openMirror()
        connect()
    } else showComputer { saved -> saved.firstOrNull { it.id == computer.id } }
}

/** Removes a computer from this phone with its mirror, drafts and notifications; the next one is shown if it was the one open. */
fun AppStore.forgetComputer(id: String) = sessionAction(queue = true) {
    transitions.withLock {
        val userId = state.value.account.userId
        val removed = state.value.computers.firstOrNull { it.id == id } ?: return@withLock
        val shown = state.value.computer?.id == id
        // The shown one's link takes this phone's push off it; another one is reached just for that.
        if (shown) retireContext(Retirement.Computer)
        else {
            NotificationDisplay.cancelComputer(getApplication(), local.referenceId, id, tasksOnly = false)
            unregisterElsewhere(removed)
        }
        asked.add(id)
        changeComputers { saved -> saved.filter { it.id != id } }
        withContext(Dispatchers.IO) { local.forgetComputerFiles(id) }
        if (shown) loadContext(userId)
    }
}

/**
 * Best effort, in the background: tells a computer that isn't shown to stop pushing to this phone.
 * Offline, it keeps sending until its tickets lapse; this phone drops pushes from computers it doesn't have.
 */
private fun AppStore.unregisterElsewhere(computer: Computer) {
    val store = local
    viewModelScope.launch {
        try {
            withTimeoutOrNull(15_000) {
                val identity = withContext(Dispatchers.IO) { store.identity() }
                val link = HostConnector.connect(computer, identity)
                try {
                    if (link.state.first { it is LinkState.Ready || it is LinkState.ComputerOffline || it is LinkState.Failed } is LinkState.Ready)
                        link.call("unregisterDevice")
                } finally { link.close() }
            }
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (_: Exception) { /* Expected while it's offline or turned this phone away: nothing to show. */ }
    }
}
