package com.codync.android

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.unit.dp
import com.codync.android.core.Channels
import com.codync.android.core.SignalBinding
import com.codync.android.core.SignalStatus
import com.codync.android.design.*
import kotlinx.coroutines.launch

/** One chat the Signal form is filling in. */
private data class ChatDraft(val chat: String, val botId: String, val allowed: String)

/**
 * Signal through a signal-cli daemon on the computer. The daemon owns the linked number; the host
 * calls its JSON-RPC endpoint, and the number stays in this computer's vault.
 */
@Composable internal fun SignalSection(state: AppState, store: AppStore) {
    var status by remember { mutableStateOf<SignalStatus?>(null) }
    var editing by remember { mutableStateOf(false) }
    var number by remember { mutableStateOf("") }
    var daemon by remember { mutableStateOf("http://127.0.0.1:7583") }
    var chats by remember { mutableStateOf(listOf<ChatDraft>()) }
    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    val computer = state.computers.firstOrNull()?.id
    LaunchedEffect(computer) { status = store.signalStatus() }
    val choices = state.bots.filter { it.kind != "group" && it.kind != "external" }
    val fallbackBot = choices.firstOrNull()?.id.orEmpty()

    fun act(work: suspend () -> Unit) {
        if (busy) return
        busy = true; error = null
        scope.launch {
            try { work() } catch (e: Exception) { error = e.message ?: "That did not work." } finally { busy = false }
        }
    }
    fun start() {
        val current = status
        number = current?.number.orEmpty()
        daemon = current?.daemon ?: "http://127.0.0.1:7583"
        chats = current?.bindings.orEmpty().map { ChatDraft(it.chat, it.botId, it.allowedUsers.joinToString(", ")) }
            .ifEmpty { listOf(ChatDraft("", "", "")) }
        editing = true
    }
    val clean = chats.mapNotNull { row ->
        val chat = row.chat.trim()
        val allowed = row.allowed.split(Regex("[\\s,]+")).filter { it.isNotBlank() }
        if (chat.isEmpty() && allowed.isEmpty()) null
        else SignalBinding(chat, row.botId.ifEmpty { fallbackBot }, allowed)
    }
    val ready = Channels.signalNumberOk(number) && daemon.trim().isNotEmpty() && clean.isNotEmpty()
        && clean.all { Channels.signalChatOk(it.chat) && it.botId.isNotEmpty() && it.allowedUsers.isNotEmpty()
            && it.allowedUsers.all { person -> Channels.signalPersonOk(person) } }

    CardSection(tr("Signal"), footer = tr("Link a number to signal-cli on the computer, run its daemon, then save the number and the daemon here. The number and keys stay on the computer.")) {
        val connected = status
        if (connected?.configured == true && !editing) {
            row {
                Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text(
                        "${tr(Channels.signalStateText(connected.state))} · ${connected.number.orEmpty()} · ${connected.bindings.orEmpty().size} ${tr("chats bound")}",
                        style = Type.body, color = Palette.secondary,
                    )
                    connected.bindings?.forEach { binding ->
                        Text("${binding.chat} · ${binding.allowedUsers.size} ${tr("people allowed")}",
                            style = Type.caption, color = Palette.tertiary)
                        ChannelIconRow(binding.icon) { icon ->
                            act { store.setChannelIcon("signal", binding.chat, icon); status = store.signalStatus() }
                        }
                        ChannelNotesRow("signal", binding.chat, store)
                        ChannelMemoryField(binding.chat, binding.memory) { memory ->
                            act { store.setChannelMemory("signal", binding.chat, memory); status = store.signalStatus() }
                        }
                    }
                    if (connected.state.startsWith("notRunning")) {
                        connected.setup?.let { Text(it, style = Type.footnote, color = Palette.text) }
                    }
                    SecondaryButton(tr("Change Signal settings"), enabled = !busy) { start() }
                    SecondaryButton(tr("Disconnect Signal"), enabled = !busy) {
                        act { status = store.disconnectSignal() }
                    }
                }
            }
        } else if (connected == null && !editing) {
            row { Text(tr("Loading…"), style = Type.body, color = Palette.secondary) }
        } else {
            if (editing && chats.isEmpty()) start()
            row {
                Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    status?.setup?.let { Text(it, style = Type.footnote, color = Palette.secondary) }
                    Text(tr("Signal number"), style = Type.caption, color = Palette.secondary)
                    PlainInput(number, { number = it }, "+15551234567")
                    Text(tr("signal-cli daemon"), style = Type.caption, color = Palette.secondary)
                    PlainInput(daemon, { daemon = it }, "http://127.0.0.1:7583")
                    for ((index, chat) in chats.withIndex()) {
                        Text(tr("Phone number or group"), style = Type.caption, color = Palette.secondary)
                        PlainInput(chat.chat, { value ->
                            chats = chats.toMutableList().also { it[index] = chat.copy(chat = value) }
                        }, "+15551234567 or group:<id>")
                        ChoicePicker(chat.botId.ifEmpty { fallbackBot }, choices.map { it.id to it.name }) { botId ->
                            chats = chats.toMutableList().also { it[index] = chat.copy(botId = botId) }
                        }
                        Text(tr("Allowed Signal numbers"), style = Type.caption, color = Palette.secondary)
                        PlainInput(chat.allowed, { value ->
                            chats = chats.toMutableList().also { it[index] = chat.copy(allowed = value) }
                        }, "+15557654321, or a UUID")
                        if (chats.size > 1) {
                            SecondaryButton(tr("Remove chat")) { chats = chats.filterIndexed { i, _ -> i != index } }
                        }
                    }
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        SecondaryButton(tr("Add chat")) { chats = chats + ChatDraft("", "", "") }
                        SecondaryButton(if (busy) tr("Connecting…") else tr("Connect Signal"), enabled = !busy && ready) {
                            act {
                                status = store.setSignal(number, daemon, clean)
                                editing = false
                            }
                        }
                        if (status?.configured == true) {
                            SecondaryButton(tr("Cancel"), enabled = !busy) { editing = false }
                        }
                    }
                }
            }
        }
        error?.let { Text(it, style = Type.caption, color = Palette.danger) }
    }
}
