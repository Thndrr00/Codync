package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import com.codync.android.core.LocalApps
import com.codync.android.design.*
import kotlinx.coroutines.launch

/**
 * The computer-wide ceiling on local mini-apps: a safeguard on this computer, not a plan. Only a new
 * app takes a slot, and a bot may always update an app it already made (iOS `LocalAppsSheet`).
 */
@Composable internal fun LocalAppsSheet(state: AppState, store: AppStore) {
    var apps by remember { mutableStateOf<LocalApps?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    // The count follows the bots: it is read again when this sheet opens on another computer.
    LaunchedEffect(state.computers.firstOrNull()?.id) {
        apps = runCatching { store.localApps() }.getOrElse {
            error = it.message ?: "That did not work."
            null
        }
    }
    fun save(limit: Int) {
        if (busy || limit == apps?.limit) return
        busy = true
        error = null
        scope.launch {
            try {
                store.setLocalAppsLimit(limit)
                apps = store.localApps()
            } catch (e: Exception) {
                error = e.message ?: "That did not work."
            } finally {
                busy = false
            }
        }
    }
    val current = apps
    val failure = error
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader(tr("Local apps")) { if (busy || (current == null && failure == null)) Spinner() }
        CardForm {
            if (current == null) {
                // A failed read stands in for the card, since there is no limit to show.
                CardSection {
                    row {
                        if (failure != null) Text(tr(failure), style = Type.body, color = Palette.danger)
                        else Text(tr("Loading…"), style = Type.body, color = Palette.secondary)
                    }
                }
            } else {
                CardSection(footer = tr("A local safeguard on this computer, not a plan: the bots here may keep up to this many mini-apps between them. Changing an app a bot already made never counts, and deleting one makes room.")) {
                    row {
                        ValueRow(tr("Apps on this computer")) {
                            Text(tr("{used} of {limit}").replace("{used}", current.count.toString())
                                .replace("{limit}", current.limit.toString()), style = Type.body, color = Palette.secondary)
                        }
                    }
                    row { ValueRow(tr("Bots that made apps"), current.botsWithApps.toString()) }
                    row {
                        ValueRow(tr("Local app limit")) {
                            ChoicePicker(current.limit, current.choices.map { it to it.toString() }, { save(it) },
                                enabled = !busy, label = tr("Local app limit"))
                        }
                    }
                }
                // A limit the computer refused: the card keeps what it holds and the reason shows under it.
                failure?.let { message -> Text(tr(message), style = Type.caption, color = Palette.danger) }
            }
        }
    }
}
