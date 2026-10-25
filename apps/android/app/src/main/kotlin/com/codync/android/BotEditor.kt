package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import androidx.compose.ui.text.style.TextOverflow
import com.codync.android.core.BotDraft
import com.codync.android.core.LinkState
import com.codync.android.design.*
import com.kyant.shapes.RoundedRectangle
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.jsonPrimitive

/** Create or edit a bot in a sheet, as iOS `BotEditorView`: the settings form plus Close and Save. */
@Composable fun BotEditor(state: AppState, initial: BotDraft, store: AppStore) {
    val base = remember(state.generation, initial.id) { initial }
    var draft by remember(state.generation, initial.id) { mutableStateOf(base) }
    var folders by remember { mutableStateOf(false) }
    var viewingSkill by remember { mutableStateOf<SkillRef?>(null) }
    var tab by remember { mutableStateOf(InspectorTab.Settings) }
    val online = state.actualConnection is LinkState.Ready
    val saving = "editor" in state.busy
    val dismiss = LocalDismissModal.current
    val isNew = draft.id == null
    val bot = state.bots.firstOrNull { it.id == draft.id }
    // A new bot starts on an installed agent we know, with every connector and skill on.
    LaunchedEffect(state.hello, state.plugins) { if (isNew) draft = fillDefaults(draft, state) }
    LaunchedEffect(online) { if (online) store.refreshPlugins() }
    Column(Modifier.fillMaxSize()) {
        ModalHeader(if (isNew) "New bot" else "Bot") {
            if (tab == InspectorTab.Settings) {
                if (saving) Box(Modifier.size(36.dp), contentAlignment = Alignment.Center) { Spinner() }
                else IconButton(if (isNew) "Create" else "Save", "checkmark", enabled = draft.isValid && online) {
                    val saved = bot?.let { draft.rebased(base, BotDraft.from(it)) } ?: draft
                    store.saveBot(saved) { dismiss() }
                }
            }
        }
        // An existing bot's settings, memory and routines sit behind one inspector (idea 33).
        if (!isNew) SegmentedChoice(tab, InspectorTab.entries.map { it to it.label }, { tab = it },
            Modifier.padding(horizontal = 20.dp).padding(bottom = 12.dp))
        when (tab) {
            InspectorTab.Settings -> BotSettingsForm(state, store, draft, { draft = it }, { viewingSkill = it }) { folders = true }
            InspectorTab.Memory -> Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(20.dp)) {
                draft.id?.let { MemoryCard(it, state, store) }
            }
            InspectorTab.Routines -> bot?.let { RoutinesScreen(it, state, store, null, embedded = true) }
        }
    }
    CodyncSheet(folders, { folders = false }) {
        FolderPicker(draft.cwd.ifEmpty { state.hello?.get("home")?.jsonPrimitive?.contentOrNull }, online, store::listDirs) {
            draft = draft.copy(cwd = it); folders = false
        }
    }
    CodyncSheet(viewingSkill, { viewingSkill = null }) { SkillSheet(it, state, store) }
}

private enum class InspectorTab(val label: String) { Settings("Settings"), Memory("Memory"), Routines("Routines") }

/** Picks an installed agent we know; connectors and skills start on once the computer has listed them. */
private fun fillDefaults(draft: BotDraft, state: AppState): BotDraft {
    val backends = backends(state)
    var next = draft
    if (backends.none { it.text("id") == draft.backend && it.available }) backends.firstOrNull { it.available && it.curated }?.text("id")
        ?.let { if (it != next.backend) next = next.copy(backend = it, account = null, model = null, modelOptions = emptyMap()) }
    val plugins = state.plugins
    if (plugins != null && next.connectors == null && plugins.connectors.isNotEmpty()) next = next.copy(connectors = plugins.connectors.map { it.id })
    if (plugins != null && next.skills.isNullOrEmpty() && plugins.skills.isNotEmpty() && draft.skills.isNullOrEmpty()) next = next.copy(skills = plugins.skills.map { it.id })
    return next
}

internal fun backends(state: AppState): List<JsonObject> = (state.hello?.get("backends") as? JsonArray).orEmpty().mapNotNull { it as? JsonObject }
internal val JsonObject.available get() = this["available"]?.jsonPrimitive?.booleanOrNull == true
internal val JsonObject.curated get() = this["curated"]?.jsonPrimitive?.booleanOrNull != false

/** Bot settings: a big avatar, then sections in the Settings style (`CardSection`). */
@Composable private fun BotSettingsForm(state: AppState, store: AppStore, draft: BotDraft, change: (BotDraft) -> Unit, viewSkill: (SkillRef) -> Unit,
    chooseFolder: () -> Unit) {
    val online = state.actualConnection is LinkState.Ready
    val screen = state.screen
    val bot = state.bots.firstOrNull { it.id == draft.id }
    var addingFallback by remember { mutableStateOf(false) }
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(20.dp), verticalArrangement = Arrangement.spacedBy(30.dp)) {
        Column(Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(16.dp)) {
            Box(Modifier.clip(RoundedRectangle(24.dp)).background(Palette.bubbleAgent).padding(18.dp)) {
                CharacterAvatar(draft.avatarShape, draft.avatarColor, 96.dp)
            }
            AvatarPicker(draft.avatarShape, draft.avatarColor, { change(draft.copy(avatarShape = it)) }, { change(draft.copy(avatarColor = it)) })
        }
        CardSection("Profile") {
            // A new bot is named from its first conversations; rename it any time after.
            if (draft.id != null) row { Field("Name") { FieldBox(draft.name, { change(draft.copy(name = it)) }, "Name") } }
            row {
                Field("Standing instructions", "Rules that always apply. Put task-specific requests in the chat instead.") {
                    FieldBox(draft.description, { change(draft.copy(description = it)) }, "e.g. Reviews PRs. Never pushes without asking.", 3, 8)
                }
            }
        }
        CardSection("Agent") {
            bot?.agentStep?.let { step -> row { FallbackBanner(state, store, bot, step) } }
            row {
                Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    ValueRow("Computer", state.computer?.name ?: "Your computer")
                    if (!online) Text(if (draft.id == null) "Connect this computer to create the bot." else "Reconnect to this computer to save settings.",
                        style = Type.caption, color = Palette.warning)
                }
            }
            row {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    val agents = backends(state).map { info ->
                        val id = info.text("id").orEmpty()
                        id to ((info.text("name") ?: id) + if (!info.available) " (not installed)" else "")
                    } + ("custom" to "Custom command")
                    ValueRow("Agent") {
                        ChoicePicker(draft.backend, agents, { if (it != draft.backend) change(draft.withAgent(it).copy(account = null)) },
                            label = "Agent")
                    }
                    backends(state).firstOrNull { it.text("id") == draft.backend && !it.available }?.text("installHint")?.takeIf(String::isNotBlank)?.let {
                        Text(it, style = Type.caption, color = Palette.warning)
                    }
                    if (draft.backend == "custom") FieldBox(draft.command.orEmpty(), { change(draft.copy(command = it.ifEmpty { null })) },
                        "ACP command, e.g. my-agent --acp", style = Type.callout.monospaced, capitalize = false)
                }
            }
            if (draft.backend == "antigravity") row {
                AgentAccountPicker(draft.backend, draft.account, online,
                    "${state.generation}/${state.computer?.id}/${draft.id}",
                    { store.agentAccounts(draft.backend) }, { store.authenticateAgentAccount(draft.backend) }) {
                    change(draft.copy(account = it, model = null, modelOptions = emptyMap()))
                }
            }
            row {
                key(state.generation, state.computer?.id, draft.backend, draft.account) {
                    AgentModelPicker(state, { backend -> store.agentModels(backend, draft.account) }, draft.backend, draft.model, draft.modelOptions, draft.fastMode ?: false, bot,
                        { model, options -> change(draft.copy(model = model, modelOptions = options)) }, { change(draft.copy(fastMode = it)) })
                }
            }
            if (draft.fallbacks != null) {
                row { FallbackList(state, draft.backend, draft.fallbacks.orEmpty(), bot, { change(draft.copy(fallbacks = it)) }, { addingFallback = true }) }
                if (draft.selfFallbacks != null) row {
                    SwitchRow("Let the bot change its fallbacks", "It can set this list itself, for example when you ask it to.",
                        draft.selfFallbacks ?: true, { change(draft.copy(selfFallbacks = it)) })
                }
            }
            row {
                ValueRow("Workspace", detail = if (draft.cwd.isEmpty()) "This bot has its own space for files. It can work in other folders when you ask."
                    else "This project is the default starting folder. The bot can work elsewhere when you ask.") {
                    DropdownMenu({ listOf(
                        MenuItem("Personal workspace", selected = draft.cwd.isEmpty()) { change(draft.copy(cwd = "")) },
                        MenuItem("Choose project folder…", selected = draft.cwd.isNotEmpty(), action = chooseFolder),
                    ) }, label = "Workspace") {
                        Row(Modifier.outline().pill(androidx.compose.ui.graphics.Color.Transparent), verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                            Text(if (draft.cwd.isEmpty()) "Personal" else draft.cwd.trimEnd('/').substringAfterLast('/').ifEmpty { "/" },
                                style = Type.body, color = Palette.text, maxLines = 1, overflow = TextOverflow.Ellipsis)
                            Symbol("chevron.down", size = 13.dp, tint = Palette.secondary)
                        }
                    }
                }
            }
            row {
                ValueRow("Permissions", detail = if (draft.permission == "auto")
                    "Tool requests are approved automatically. Your agent's own settings (like Claude Code's permission rules) still apply."
                    else "You'll get an approval card and a notification whenever the agent asks.") {
                    ChoicePicker(draft.permission, listOf("ask" to "Ask me", "auto" to "Approve automatically"), { change(draft.copy(permission = it)) },
                        label = "Permissions")
                }
            }
        }
        CardSection("Activity") {
            row { SwitchRow("Notifications", "Get notified when this bot finishes or needs you", draft.notify != false, { change(draft.copy(notify = it)) }) }
            row {
                SwitchRow("Chief of Staff (Coordinator)",
                    "Supervises stalled or failing bots in teams and groups, with automatic triage and up to 2 retries per incident.",
                    draft.isChief, { change(draft.copy(isChief = it)) })
            }
            if (screen != null) row {
                SwitchRow("Use the computer", when {
                    !screen.controlReady -> "Let this bot use apps. On that computer, open Codync → Settings → Computer access to set up permissions first."
                    screen.enabled -> "Let this bot see the screen and use the mouse and keyboard. You can watch and take over from your phone."
                    screen.computerUse -> "Let this bot see the screen and use the mouse and keyboard."
                    else -> "Let this bot see the screen and use the mouse and keyboard. On that computer, open Codync → Settings → Computer access to set up permissions first."
                },
                    draft.computer, { change(draft.copy(computer = it)) })
            }
            // Only a host with team management sends the setting (desktop `BotEditorView`).
            if (state.bots.any { it.manageTeam != null }) row {
                SwitchRow("Manage the team",
                    "Let this bot create bots and change its own setup and the bots it created. The bots it creates can too.",
                    draft.manageTeam, { change(draft.copy(manageTeam = it)) })
            }
        }
        val plugins = state.plugins
        PluginToggles("Connectors", "No connectors yet. Add GitHub, Linear, Notion and more from Extensions.",
            plugins?.connectors.orEmpty().map { Triple(it.id, it.name, it.command ?: it.url.orEmpty()) },
            draft.connectors ?: plugins?.connectors.orEmpty().map { it.id }, { change(draft.copy(connectors = it)) })
        PluginToggles("Skills", "No skills yet. Get some from Extensions, or write your own.",
            plugins?.skills.orEmpty().map { Triple(it.id, it.name, it.description) }, draft.skills.orEmpty(), { change(draft.copy(skills = it)) }) { id ->
            plugins?.skills?.firstOrNull { it.id == id }?.let { viewSkill(SkillRef(it.id, it.name, it.description)) }
        }
        draft.id?.let { MemoryCard(it, state, store) }
        draft.id?.let { DocumentsCard(it, state, store) }
        draft.id?.let { ComputerPanelCard(it, state, store) { enabled, permissions -> change(draft.copy(computer = enabled, computerPermissions = permissions)) } }
        state.error?.let { Text(it, style = Type.footnote, color = Palette.danger) }
    }
    CodyncSheet(addingFallback, { addingFallback = false }) {
        AddFallbackSheet(state, store, draft.backend, draft.fallbacks.orEmpty()) {
            change(draft.copy(fallbacks = draft.fallbacks.orEmpty() + it))
            addingFallback = false
        }
    }
}
