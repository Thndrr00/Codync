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
import com.codync.android.core.fill
import com.codync.android.design.*
import kotlinx.coroutines.launch

/**
 * The computer-wide ceiling on local mini-apps: a safeguard on this computer, not a plan. Only a new
 * app takes a slot, and a bot may always update an app it already made.
 */
@Composable internal fun LocalAppsSheet(state: AppState, store: AppStore) {
    var apps by remember { mutableStateOf<LocalApps?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    val computer = state.computers.firstOrNull()?.id
    LaunchedEffect(computer) {
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
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader(tr("Local apps"))
        CardForm {
            val current = apps
            CardSection(footer = tr("A local safeguard on this computer, not a plan: the bots here may keep up to this many mini-apps between them. Changing an app a bot already made never counts, and deleting one makes room.")) {
                if (current == null) row { Text(tr("Loading…"), style = Type.body, color = Palette.secondary) }
                else {
                    row {
                        ValueRow(tr("Apps on this computer")) {
                            Text(fill(tr("{used} of {limit}"), "used" to current.count.toString(), "limit" to current.limit.toString()),
                                style = Type.body, color = Palette.secondary)
                        }
                    }
                    row {
                        ValueRow(tr("Local app limit")) {
                            ChoicePicker(current.limit, current.choices.map { it to it.toString() }, { save(it) }, enabled = !busy,
                                label = tr("Local app limit"))
                        }
                    }
                }
            }
            error?.let { message -> row { Text(message, style = Type.footnote, color = Palette.danger) } }
            if (busy) row { Text(tr("Saving…"), style = Type.footnote, color = Palette.secondary) }
        }
    }
}
