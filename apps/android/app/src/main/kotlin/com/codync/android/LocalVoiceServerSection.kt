package com.codync.android

import androidx.compose.animation.animateContentSize
import androidx.compose.foundation.layout.*
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.codync.android.core.*
import com.codync.android.design.*
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch

/**
 * The local voice server the computer's OpenAI speech can go through (iOS `LocalVoiceServerSection`):
 * an OpenAI-compatible server the person runs there. Codync runs no model itself. The setting is the
 * computer's, so the phone shows it and can test it, like the voice keys; changing it happens there.
 */
@Composable internal fun LocalVoiceServerSection(state: AppState, store: AppStore) {
    val host = state.hostName
    val scope = rememberCoroutineScope()
    /** For the test's answer, which is built outside composition. */
    val t = translator()
    /** null until the computer answers. */
    var saved by remember { mutableStateOf<VoiceServer?>(null) }
    var busy by remember { mutableStateOf(false) }
    var result by remember { mutableStateOf<Pair<Boolean, String>?>(null) }

    LaunchedEffect(Unit) {
        if (state.actualConnection !is LinkState.Ready) return@LaunchedEffect
        try { saved = store.voiceServer() } catch (cancelled: CancellationException) { throw cancelled }
        catch (error: Exception) { result = false to (error.message ?: "Couldn't reach $host.") }
    }

    fun test() {
        if (busy) return
        busy = true; result = null
        scope.launch {
            try {
                val answer = store.testVoiceServer()
                result = answer.ok to if (answer.ok) t("Server works") else "${t("The server didn't answer")}: ${answer.message}"
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) { result = false to (error.message ?: "Couldn't reach $host.") }
            finally { busy = false }
        }
    }

    val footer = tr("Runs on this computer: Codync sends your dictation and spoken replies there, and no audio goes to OpenAI.")
    CardSection(tr("Local voice server"), footer, Modifier.animateContentSize(Motion.layoutSize)) {
        val server = saved
        if (server == null) {
            row { Spinner() }
        } else {
            if (server.custom) {
                row { ValueRow("Server URL", server.effective) }
                if (server.transcribeModel.isNotEmpty()) row { ValueRow("Dictation model", server.transcribeModel) }
                if (server.speechModel.isNotEmpty()) row { ValueRow("Speech model", server.speechModel) }
            } else {
                row {
                    Text(tr("No local voice server is set. Add the URL of the server on this computer."),
                        style = Type.body, color = Palette.secondary)
                }
            }
            row {
                Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    Text(tr("Set on the computer. Add or change it in Codync on $host."), Modifier.weight(1f),
                        style = Type.caption, color = Palette.secondary)
                    if (busy) Spinner() else SecondaryButton(tr("Test server"), enabled = server.custom, action = ::test)
                }
            }
            result?.let { (ok, text) ->
                row { IconLabel(text, if (ok) "checkmark.circle.fill" else "exclamationmark.circle.fill", if (ok) Palette.secondary else Palette.danger) }
            }
        }
    }
}
