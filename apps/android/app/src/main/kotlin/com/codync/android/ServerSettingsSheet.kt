package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import com.codync.android.core.ServerSettings
import com.codync.android.design.*

/**
 * Settings that belong to this computer rather than to one bot: Personality (how its agents post
 * updates between steps), the Claude Code setup switch, and who may control it over remote desktop.
 * "Stop all" is on the computer itself: only its own app may end every session.
 */
@Composable
internal fun ServerSettingsSheet(store: AppStore) {
    var settings by remember { mutableStateOf<ServerSettings?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }
    fun save(next: ServerSettings) {
        error = null
        store.action("server-settings") {
            busy = true
            try {
                settings = store.setServerSettings(next)
            } catch (failure: Exception) {
                error = failure.message
            } finally {
                busy = false
            }
        }
    }
    LaunchedEffect(Unit) {
        runCatching { store.serverSettings() }
            .onSuccess { settings = it }
            .onFailure { error = it.message }
    }
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader("Agents on this computer")
        CardForm {
            val current = settings
            if (current == null) {
                CardSection { row { Text(tr("Loading…"), style = Type.body, color = Palette.secondary) } }
            } else {
                CardSection(
                    "Personality",
                    footer = "How the agents on this computer post updates between their steps. Natural writes a short note only when a finding changes the plan or before a long step, then an answer that starts with the verdict. Classic shows their own text as it comes.",
                ) {
                    row {
                        ValueRow("Updates between steps") {
                            ChoicePicker(
                                current.responseStyle,
                                listOf("natural" to tr("Natural"), "classic" to tr("Classic")),
                                { style -> save(current.copy(responseStyle = style)) },
                                enabled = !busy,
                                label = tr("Updates between steps"),
                            )
                        }
                    }
                }
                CardSection(
                    "Claude Code",
                    footer = "Off by default: your agents do not load your CLAUDE.md, skills, agents, hooks or plugins from ~/.claude, so each turn sends a shorter prompt. Your sign-in settings still apply, and the files of an agent's workspace still load.",
                ) {
                    row {
                        CodyncSwitch(current.claudeUserSetup, { on -> save(current.copy(claudeUserSetup = on)) }, Modifier.fillMaxWidth()) {
                            Column(Modifier.weight(1f)) {
                                Text(tr("Use my Claude Code setup"), style = Type.body, color = Palette.text)
                                Text(
                                    tr("Give Claude agents your own Claude Code configuration. Turn this on if they should use your CLAUDE.md, skills or hooks."),
                                    style = Type.caption,
                                    color = Palette.secondary,
                                )
                            }
                        }
                    }
                }
                CardSection(
                    "Remote desktop",
                    footer = "Every device you paired may control this computer. Codync has one owner and no member roles yet, so \"Owners and admins\" reads as every device; \"Only this computer\" never lets a phone control it. Stop all is on the computer itself.",
                ) {
                    row {
                        ValueRow("Who can control") {
                            ChoicePicker(
                                current.remoteControl.access,
                                listOf(
                                    "everyone" to tr("Every device"),
                                    "admins" to tr("Owners and admins"),
                                    "owner" to tr("Only this computer"),
                                ),
                                { access -> save(current.copy(remoteControl = current.remoteControl.copy(access = access))) },
                                enabled = !busy,
                                label = tr("Who can control"),
                            )
                        }
                    }
                    row {
                        CodyncSwitch(current.remoteControl.askOnHost, { on ->
                            save(current.copy(remoteControl = current.remoteControl.copy(askOnHost = on)))
                        }, Modifier.fillMaxWidth()) {
                            Column(Modifier.weight(1f)) {
                                Text(tr("Ask on this computer"), style = Type.body, color = Palette.text)
                                Text(
                                    tr("Allow or deny each session here. A session nobody allows in 30 seconds does not start."),
                                    style = Type.caption,
                                    color = Palette.secondary,
                                )
                            }
                        }
                    }
                }
            }
            error?.let { message -> row { Text(message, style = Type.footnote, color = Palette.danger) } }
        }
    }
}
