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
import com.codync.android.core.MatrixBinding
import com.codync.android.core.MatrixStatus
import com.codync.android.design.*
import kotlinx.coroutines.launch

/** One room the Matrix form is filling in. */
private data class RoomDraft(val roomId: String, val botId: String, val allowed: String)

/**
 * The Matrix connection: the homeserver and the bot's account, its bound rooms and the people allowed
 * in each. The session stays on the computer; the phone only sends credentials when connecting.
 */
@Composable internal fun MatrixSection(state: AppState, store: AppStore) {
    var status by remember { mutableStateOf<MatrixStatus?>(null) }
    var editing by remember { mutableStateOf(false) }
    var homeserver by remember { mutableStateOf("") }
    var user by remember { mutableStateOf("") }
    var accessToken by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    var rooms by remember { mutableStateOf(listOf<RoomDraft>()) }
    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    val computer = state.computers.firstOrNull()?.id
    LaunchedEffect(computer) { status = store.matrixStatus() }
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
        homeserver = current?.homeserver.orEmpty()
        user = current?.userId.orEmpty()
        accessToken = ""
        password = ""
        rooms = current?.bindings.orEmpty().map { RoomDraft(it.roomId, it.botId, it.allowedUsers.joinToString(", ")) }
            .ifEmpty { listOf(RoomDraft("", "", "")) }
        editing = true
    }
    val clean = rooms.mapNotNull { row ->
        val room = row.roomId.trim()
        val allowed = row.allowed.split(Regex("[\\s,]+")).filter { it.isNotBlank() }
        if (room.isEmpty() && allowed.isEmpty()) null
        else MatrixBinding(room, row.botId.ifEmpty { fallbackBot }, allowed)
    }
    val ready = Channels.matrixHomeserverOk(homeserver) && Channels.matrixUserOk(user) && clean.isNotEmpty()
        && clean.all { Channels.matrixRoomOk(it.roomId) && it.botId.isNotEmpty() && it.allowedUsers.isNotEmpty()
            && it.allowedUsers.all { id -> Channels.matrixUserOk(id) } }
        && (accessToken.trim().isNotEmpty() || password.isNotEmpty() || status?.configured == true)

    CardSection(tr("Matrix"), footer = tr("Make a Matrix account for the bot, invite it to the rooms, and paste its homeserver and credentials here. Leave the access token empty to sign in with the password once: the computer keeps the session.")) {
        val connected = status
        if (connected?.configured == true && !editing) {
            row {
                Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text(
                        "${tr(Channels.matrixStateText(connected.state))} · ${connected.userId.orEmpty()} · ${connected.homeserver.orEmpty()} · ${connected.bindings.orEmpty().size} ${tr("rooms bound")}",
                        style = Type.body, color = Palette.secondary,
                    )
                    connected.bindings?.forEach { binding ->
                        Text("${binding.roomId} · ${binding.allowedUsers.size} ${tr("people allowed")}",
                            style = Type.caption, color = Palette.tertiary)
                        ChannelIconRow(binding.icon) { icon ->
                            act { store.setChannelIcon("matrix", binding.roomId, icon); status = store.matrixStatus() }
                        }
                        ChannelNotesRow("matrix", binding.roomId, store)
                        ChannelRoutinesRow("matrix", binding.roomId, binding.botId, store)
                        ChannelMemoryField(binding.roomId, binding.memory) { memory ->
                            act { store.setChannelMemory("matrix", binding.roomId, memory); status = store.matrixStatus() }
                        }
                    }
                    SecondaryButton(tr("Change Matrix settings"), enabled = !busy) { start() }
                    SecondaryButton(tr("Disconnect Matrix"), enabled = !busy) {
                        act { status = store.disconnectMatrix() }
                    }
                }
            }
        } else if (connected == null && !editing) {
            row { Text(tr("Loading…"), style = Type.body, color = Palette.secondary) }
        } else {
            if (editing && rooms.isEmpty()) start()
            row {
                Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    Text(tr("Matrix homeserver"), style = Type.caption, color = Palette.secondary)
                    PlainInput(homeserver, { homeserver = it }, "https://matrix.example.org")
                    Text(tr("Matrix user"), style = Type.caption, color = Palette.secondary)
                    PlainInput(user, { user = it }, "@bot:example.org")
                    Text(tr("Access token"), style = Type.caption, color = Palette.secondary)
                    PlainInput(accessToken, { accessToken = it }, tr("Leave empty to keep the saved session"), secret = true)
                    Text(tr("Password"), style = Type.caption, color = Palette.secondary)
                    PlainInput(password, { password = it }, tr("Used once to sign in, never saved"), secret = true)
                    for ((index, room) in rooms.withIndex()) {
                        Text(tr("Room id or alias"), style = Type.caption, color = Palette.secondary)
                        PlainInput(room.roomId, { value ->
                            rooms = rooms.toMutableList().also { it[index] = room.copy(roomId = value) }
                        }, "!room:example.org")
                        ChoicePicker(room.botId.ifEmpty { fallbackBot }, choices.map { it.id to it.name }) { botId ->
                            rooms = rooms.toMutableList().also { it[index] = room.copy(botId = botId) }
                        }
                        Text(tr("Allowed Matrix users"), style = Type.caption, color = Palette.secondary)
                        PlainInput(room.allowed, { value ->
                            rooms = rooms.toMutableList().also { it[index] = room.copy(allowed = value) }
                        }, "@ana:example.org, @beto:example.org")
                        if (rooms.size > 1) {
                            SecondaryButton(tr("Remove room")) { rooms = rooms.filterIndexed { i, _ -> i != index } }
                        }
                    }
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        SecondaryButton(tr("Add room")) { rooms = rooms + RoomDraft("", "", "") }
                        SecondaryButton(if (busy) tr("Signing in…") else tr("Connect Matrix"), enabled = !busy && ready) {
                            act {
                                status = store.setMatrix(homeserver, user, accessToken, password, clean)
                                editing = false; accessToken = ""; password = ""
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
