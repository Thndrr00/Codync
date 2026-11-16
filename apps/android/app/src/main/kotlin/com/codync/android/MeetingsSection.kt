package com.codync.android

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import com.codync.android.design.*
import java.io.ByteArrayOutputStream
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.launch

/** 20 seconds of 16 kHz mono PCM16, the most one transcription request takes. */
private const val MEETING_PIECE_BYTES = 16_000 * 2 * 20

/**
 * Meeting notes on the phone (iOS `MeetingsSectionView`): record the microphone, the computer writes
 * the transcript, and a bot drafts the minutes. The audio never stays on the phone.
 */
@Composable
internal fun MeetingsSection(store: AppStore, onError: (String?) -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var notes by remember { mutableStateOf<List<MeetingSummary>>(emptyList()) }
    var title by remember { mutableStateOf("") }
    var active by remember { mutableStateOf<MeetingStart?>(null) }
    var elapsed by remember { mutableStateOf(0) }
    var sending by remember { mutableStateOf(false) }
    var transcribing by remember { mutableStateOf(false) }
    var open by remember { mutableStateOf<MeetingNote?>(null) }
    var busy by remember { mutableStateOf(false) }
    val mic = remember { AndroidMic(context) }
    var pieces by remember { mutableStateOf(Channel<ByteArray>(Channel.UNLIMITED)) }
    var collector by remember { mutableStateOf<Job?>(null) }

    suspend fun load() {
        try {
            notes = store.meetingNotes()
            onError(null)
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (failure: Exception) {
            onError(failure.message)
        }
    }

    LaunchedEffect(Unit) { load() }

    LaunchedEffect(active) {
        elapsed = 0
        while (active != null) {
            kotlinx.coroutines.delay(1_000)
            elapsed += 1
        }
    }

    DisposableEffect(Unit) {
        onDispose { scope.launch { mic.stop() } }
    }

    fun start() {
        if (active != null) return
        busy = true
        scope.launch {
            try {
                val started = store.startMeeting(title.trim())
                active = started
                title = ""
                val channel = Channel<ByteArray>(Channel.UNLIMITED)
                pieces = channel
                collector = scope.launch {
                    val buffer = ByteArrayOutputStream()
                    var last: Job? = null
                    suspend fun flush() {
                        if (buffer.size() == 0) return
                        val bytes = buffer.toByteArray()
                        buffer.reset()
                        last?.join()
                        sending = true
                        last = scope.launch {
                            try {
                                store.appendMeetingAudio(started.id, bytes)
                            } catch (failure: Exception) {
                                onError(failure.message)
                            } finally {
                                sending = false
                            }
                        }
                    }
                    while (true) {
                        val pcm = channel.receiveCatching().getOrNull() ?: break
                        buffer.write(pcm)
                        if (buffer.size() >= MEETING_PIECE_BYTES) flush()
                    }
                    flush()
                    last?.join()
                }
                mic.start({ pcm, _ -> channel.trySend(pcm) }, { onError("Couldn't use the microphone.") })
                onError(null)
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (failure: Exception) {
                active = null
                collector?.cancel()
                collector = null
                onError(failure.message)
            } finally {
                busy = false
            }
        }
    }

    fun stop() {
        val started = active ?: return
        transcribing = true
        scope.launch {
            try {
                mic.stop()
                pieces.close()
                collector?.join()
                collector = null
                val note = store.finishMeeting(started.id)
                open = note
                active = null
                load()
                onError(null)
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (failure: Exception) {
                onError(failure.message)
            } finally {
                transcribing = false
            }
        }
    }

    fun discard() {
        val started = active ?: return
        scope.launch {
            try {
                mic.stop()
                pieces.close()
                collector?.join()
                collector = null
                store.cancelMeeting(started.id)
                active = null
                open = null
                onError(null)
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (failure: Exception) {
                onError(failure.message)
            }
        }
    }

    CardSection(
        "Meetings",
        "Record the microphone here. The computer writes it down and a bot can draft the minutes; keep Codync open while it records.",
    ) {
        val started = active
        if (started != null) {
            row {
                Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                        Text(started.title, style = Type.body, color = Palette.text)
                        Text(
                            if (sending) "Sending the last piece…" else meetingTime(elapsed),
                            style = Type.footnote,
                            color = Palette.secondary,
                        )
                    }
                    SecondaryButton(if (transcribing) "Writing it down…" else "Stop", enabled = !transcribing) { stop() }
                }
                LinkButton("Discard this recording") { discard() }
            }
        } else {
            row {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    FieldBox(title, { title = it }, "Meeting title")
                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                        SecondaryButton("Record a meeting", enabled = !busy) { start() }
                    }
                }
            }
        }

        for (note in notes) row {
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    Text(note.title, style = Type.body, color = Palette.text)
                    Text(
                        "${meetingWhen(note.startedAt)} · ${note.chars} characters${if (note.hasMinutes) " · Minutes" else ""}",
                        style = Type.footnote,
                        color = Palette.secondary,
                    )
                }
                if (!note.hasMinutes) SecondaryButton("Minutes", enabled = !busy) { scope.launch {
                    busy = true
                    try {
                        open = store.meetingMinutes(note.id)
                        load()
                        onError(null)
                    } catch (cancelled: CancellationException) {
                        throw cancelled
                    } catch (failure: Exception) {
                        onError(failure.message)
                    } finally {
                        busy = false
                    }
                } }
                SecondaryButton(if (open?.id == note.id) "Close" else "Open") { scope.launch {
                    try {
                        open = if (open?.id == note.id) null else store.meetingNote(note.id)
                        onError(null)
                    } catch (cancelled: CancellationException) {
                        throw cancelled
                    } catch (failure: Exception) {
                        onError(failure.message)
                    }
                } }
                LinkButton("Delete") { scope.launch {
                    try {
                        store.deleteMeetingNote(note.id)
                        if (open?.id == note.id) open = null
                        load()
                    } catch (cancelled: CancellationException) {
                        throw cancelled
                    } catch (failure: Exception) {
                        onError(failure.message)
                    }
                } }
            }
        }
        if (notes.isEmpty() && !busy) row {
            Text("No meeting notes yet.", style = Type.footnote, color = Palette.secondary)
        }

        open?.let { note ->
            row {
                Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    note.minutes?.takeIf { it.isNotBlank() }?.let { minutes ->
                        Text("Minutes", style = Type.footnote, color = Palette.secondary)
                        Text(minutes, style = Type.body, color = Palette.text)
                    }
                    Text("Transcript", style = Type.footnote, color = Palette.secondary)
                    Text(note.transcript, style = Type.body, color = Palette.text)
                }
            }
        }
    }
}

private fun meetingWhen(ms: Long): String =
    SimpleDateFormat("d MMM, HH:mm", Locale.getDefault()).format(Date(ms))

private fun meetingTime(seconds: Int): String = "%d:%02d".format(seconds / 60, seconds % 60)
