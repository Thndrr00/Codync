package com.codync.android

import android.media.MediaPlayer
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.unit.dp
import com.codync.android.core.Bot
import com.codync.android.design.*
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import java.io.File
import java.util.Base64

/**
 * A short roundtable of a group's members on one topic, with the draft minutes and each turn read
 * aloud when the computer had a voice provider (iOS `MeetingView`).
 */
@Composable
internal fun TeamMeetingSheet(bot: Bot, store: AppStore) {
    var topic by remember { mutableStateOf("") }
    var meeting by remember { mutableStateOf<TeamMeeting?>(null) }
    var running by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var playing by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    val clipboard = LocalClipboardManager.current
    val context = LocalContext.current
    var player by remember { mutableStateOf<MediaPlayer?>(null) }

    fun stop() {
        player?.release()
        player = null
        playing = null
    }

    DisposableEffect(Unit) { onDispose { player?.release() } }

    fun play(turn: MeetingTurn) {
        val encoded = turn.audio ?: return
        if (playing == turn.botId) {
            stop()
            return
        }
        stop()
        try {
            val bytes = Base64.getDecoder().decode(encoded)
            val suffix = turn.audioFormat ?: "mp3"
            val file = File(context.cacheDir, "meeting-${turn.botId}.$suffix").apply { writeBytes(bytes) }
            val next = MediaPlayer().apply {
                setDataSource(file.path)
                setOnCompletionListener { stop() }
                prepare()
                start()
            }
            player = next
            playing = turn.botId
        } catch (failure: Exception) {
            error = failure.message
        }
    }

    fun start() {
        val question = topic.trim()
        if (question.isEmpty()) return
        running = true
        error = null
        scope.launch {
            try {
                meeting = store.teamMeeting(bot.id, question, voice = true)
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (failure: Exception) {
                error = failure.message
            } finally {
                running = false
            }
        }
    }

    Column(Modifier.fillMaxWidth()) {
        ModalHeader("Team meeting") { if (running) Spinner() }
        CardForm {
            CardSection("Topic",
                "The group's members answer in turn, in throwaway sessions. Nothing lands in the chat; the minutes are a draft you can copy.") {
                row {
                    FieldBox(topic, { topic = it }, "What should the team think through?", minLines = 2, maxLines = 4)
                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                        PrimaryButton(if (running) "Thinkingâ€¦" else "Start meeting", enabled = !running && topic.isNotBlank()) { start() }
                    }
                }
            }
            val current = meeting
            if (current != null) {
                CardSection("Minutes") {
                    row {
                        Text(current.minutes, style = Type.body, color = Palette.text)
                        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                            SecondaryButton("Copy minutes") { clipboard.setText(AnnotatedString(current.minutes)) }
                        }
                    }
                }
                for (turn in current.turns) CardSection(turn.name) {
                    row {
                        Text(turn.text, style = Type.body, color = Palette.text)
                        if (turn.audio != null) Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                            SecondaryButton(if (playing == turn.botId) "Stop" else "Read aloud") { play(turn) }
                        }
                    }
                }
            }
            if (error != null) Text(error ?: "", style = Type.footnote, color = Palette.danger)
        }
    }
}

