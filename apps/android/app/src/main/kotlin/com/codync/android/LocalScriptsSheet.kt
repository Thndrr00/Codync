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
import com.codync.android.core.ScriptLimit
import com.codync.android.design.*
import kotlinx.coroutines.launch

/**
 * The hourly ceiling on requests local scripts send to routines of this computer, shared by every agent.
 * Past it the local address answers 429 and the script tries again later.
 */
@Composable internal fun LocalScriptsSheet(state: AppState, store: AppStore) {
    var limit by remember { mutableStateOf<ScriptLimit?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    val computer = state.computers.firstOrNull()?.id
    LaunchedEffect(computer) {
        limit = runCatching { store.scriptLimit() }.getOrElse {
            error = it.message ?: "That did not work."
            null
        }
    }
    fun save(perHour: Int) {
        if (busy || perHour == limit?.perHour) return
        busy = true
        error = null
        scope.launch {
            try {
                store.setScriptLimit(perHour)
                limit = store.scriptLimit()
            } catch (e: Exception) {
                error = e.message ?: "That did not work."
            } finally {
                busy = false
            }
        }
    }
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader("Local scripts")
        CardForm {
            val current = limit
            CardSection(footer = tr("How many requests a script on this computer may send to routines each hour, for every agent together. Past it the address answers 429 and the script tries again later.")) {
                if (current == null) row { Text(tr("Loading…"), style = Type.body, color = Palette.secondary) }
                else row {
                    ValueRow("Requests an hour") {
                        ChoicePicker(current.perHour, current.choices.map { it to it.toString() }, { save(it) }, enabled = !busy,
                            label = tr("Requests an hour"))
                    }
                }
            }
            error?.let { message -> row { Text(message, style = Type.footnote, color = Palette.danger) } }
            if (busy) row { Text(tr("Saving…"), style = Type.footnote, color = Palette.secondary) }
        }
    }
}
