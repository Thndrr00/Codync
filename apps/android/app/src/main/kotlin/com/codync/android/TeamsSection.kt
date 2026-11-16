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
import com.codync.android.core.TeamsStatus
import com.codync.android.design.*
import kotlinx.coroutines.launch

/**
 * Microsoft Teams through an Azure Bot resource. The app secret stays on the computer; the
 * registration and its public messaging endpoint are the person's own Azure work.
 */
@Composable internal fun TeamsSection(state: AppState, store: AppStore) {
    var status by remember { mutableStateOf<TeamsStatus?>(null) }
    var editing by remember { mutableStateOf(false) }
    var appId by remember { mutableStateOf("") }
    var tenantId by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    var managed by remember { mutableStateOf(false) }
    var botId by remember { mutableStateOf("") }
    var users by remember { mutableStateOf("") }
    var tenants by remember { mutableStateOf("") }
    var memory by remember { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    val computer = state.computers.firstOrNull()?.id
    LaunchedEffect(computer) { status = store.teamsStatus() }
    val choices = state.bots.filter { it.kind != "group" && it.kind != "external" }
    val fallbackBot = choices.firstOrNull()?.id.orEmpty()
    val allowedUsers = users.split(Regex("[\\s,]+")).filter { it.isNotBlank() }
    val allowedTenants = tenants.split(Regex("[\\s,]+")).filter { it.isNotBlank() }

    fun act(work: suspend () -> Unit) {
        if (busy) return
        busy = true; error = null
        scope.launch {
            try { work() } catch (e: Exception) { error = e.message ?: "That did not work." } finally { busy = false }
        }
    }
    fun start() {
        val current = status
        appId = current?.appId.orEmpty()
        tenantId = current?.tenantId.orEmpty()
        password = ""
        managed = current?.managedIdentity == true
        botId = current?.botId.orEmpty()
        users = current?.allowedUsers.orEmpty().joinToString(", ")
        tenants = current?.allowedTenants.orEmpty().joinToString(", ")
        memory = current?.memory.orEmpty()
        editing = true
    }
    val ready = Regex("^[0-9a-fA-F-]{36}$").matches(appId.trim())
        && (managed || password.isNotEmpty() || status?.configured == true)
        && (botId.ifEmpty { fallbackBot }).isNotEmpty()
        && (allowedUsers.isNotEmpty() || allowedTenants.isNotEmpty())

    CardSection(tr("Microsoft Teams"), footer = tr("Create an Azure Bot resource, enable the Teams channel and point its messaging endpoint at this computer. Teams only reaches a public HTTPS address. The client secret stays on the computer.")) {
        val connected = status
        if (connected?.configured == true && !editing) {
            row {
                Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text(
                        "${tr(Channels.teamsStateText(connected.state))} · ${connected.appId.orEmpty()} · ${connected.allowedUsers.size} ${tr("people allowed")}",
                        style = Type.body, color = Palette.secondary,
                    )
                    Text("${tr("Messaging endpoint")}: ${connected.endpoint.orEmpty()}", style = Type.caption, color = Palette.tertiary)
                    ChannelIconRow(connected.icon) { icon ->
                        act { store.setChannelIcon("teams", "teams", icon); status = store.teamsStatus() }
                    }
                    ChannelNotesRow("teams", "teams", store)
                    ChannelMemoryField("teams", connected.memory) { value ->
                        act { store.setChannelMemory("teams", "teams", value); status = store.teamsStatus() }
                    }
                    SecondaryButton(tr("Change Teams settings"), enabled = !busy) { start() }
                    SecondaryButton(tr("Disconnect Teams"), enabled = !busy) {
                        act { status = store.disconnectTeams() }
                    }
                }
            }
        } else if (connected == null && !editing) {
            row { Text(tr("Loading…"), style = Type.body, color = Palette.secondary) }
        } else {
            row {
                Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    Text("${tr("Microsoft Teams")} · ${tr(Channels.teamsStateText(status?.state ?: "notConfigured"))}",
                        style = Type.body, color = Palette.secondary)
                    status?.registration?.let { Text(it, style = Type.footnote, color = Palette.secondary) }
                    Text(tr("Microsoft app id"), style = Type.caption, color = Palette.secondary)
                    PlainInput(appId, { appId = it }, "00000000-0000-0000-0000-000000000000")
                    Text(tr("Directory tenant"), style = Type.caption, color = Palette.secondary)
                    PlainInput(tenantId, { tenantId = it }, "contoso.onmicrosoft.com, or empty")
                    ChoicePicker(if (managed) "managed" else "secret", listOf("secret" to tr("App client secret"), "managed" to tr("Managed identity"))) {
                        managed = it == "managed"
                    }
                    if (!managed) {
                        Text(tr("Client secret"), style = Type.caption, color = Palette.secondary)
                        PlainInput(password, { password = it },
                            if (status?.configured == true) tr("Leave empty to keep the saved secret") else tr("The app registration secret"),
                            secret = true)
                    }
                    ChoicePicker(botId.ifEmpty { fallbackBot }, choices.map { it.id to it.name }) { botId = it }
                    Text(tr("People allowed"), style = Type.caption, color = Palette.secondary)
                    PlainInput(users, { users = it }, "ana@example.com, or an AAD object id, or *")
                    Text(tr("Tenants allowed"), style = Type.caption, color = Palette.secondary)
                    PlainInput(tenants, { tenants = it }, "a tenant id, or *")
                    Text(tr("What the bot knows about Teams"), style = Type.caption, color = Palette.secondary)
                    PlainInput(memory, { memory = it }, tr("Given to the bot before each Teams message"))
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        SecondaryButton(if (busy) tr("Connecting…") else tr("Connect Teams"), enabled = !busy && ready) {
                            act {
                                status = store.setTeams(appId, tenantId, password, managed, botId.ifEmpty { fallbackBot },
                                    allowedUsers, allowedTenants, memory, "")
                                editing = false; password = ""
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
