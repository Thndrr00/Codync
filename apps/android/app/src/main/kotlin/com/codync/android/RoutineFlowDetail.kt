package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.codync.android.core.Bot
import com.codync.android.core.FlowCanvas
import com.codync.android.core.FlowLink
import com.codync.android.core.FlowMedia
import com.codync.android.core.FlowRoutineCard
import com.codync.android.core.MediaAnswer
import com.codync.android.core.MediaChoice
import com.codync.android.core.RoutineFlow
import com.codync.android.core.flowStatusLabel
import com.codync.android.core.modelsOf
import com.codync.android.core.onModel
import com.codync.android.design.*
import com.kyant.shapes.RoundedRectangle

/** What the detail sheet is about: a routine, a bot or a media step (by node key), or a link (by id). */
internal sealed interface FlowSelection {
    data class OfNode(val key: String) : FlowSelection
    data class OfLink(val id: String) : FlowSelection
}

/** A bot or a media step the selected step may hand work to. */
internal data class FlowTarget(val id: String, val name: String)

/** A step's status as a small chip: words and a symbol, never color alone. */
@Composable internal fun StatusChip(status: String?) {
    val symbol = when (status) {
        "running" -> "arrow.triangle.2.circlepath"; "succeeded" -> "checkmark.circle"; "failed" -> "xmark.circle"
        "skipped" -> "minus.circle"; "cancelled" -> "stop.circle"; else -> "circle.dashed"
    }
    val tint = when (status) { "succeeded" -> Palette.added; "failed" -> Palette.danger; "running" -> Palette.accent; else -> Palette.secondary }
    Row(Modifier.clip(RoundedRectangle(50.dp)).background(Palette.text.copy(alpha = 0.07f)).padding(horizontal = 8.dp, vertical = 2.dp),
        verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
        Symbol(symbol, size = 12.dp, tint = tint)
        Text(tr(flowStatusLabel(status)), style = Type.caption, color = tint, maxLines = 1)
    }
}

/** The sheet over the canvas or the list: what the selected routine, bot, media step or link says, and the actions on it. */
@Composable internal fun FlowDetail(state: AppState, owner: Bot, canvas: FlowCanvas, bots: Map<String, Bot>,
    targets: (String) -> List<FlowTarget>, selection: FlowSelection, online: Boolean, busy: Boolean, store: AppStore,
    note: (String?) -> Unit, reselect: (FlowSelection) -> Unit, close: () -> Unit) {
    val unknownBot = tr("Unknown bot")
    val name = { id: String -> bots[id]?.name ?: unknownBot }
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader(when (selection) {
            is FlowSelection.OfLink -> tr("Link")
            is FlowSelection.OfNode -> when {
                selection.key.startsWith("routine:") -> tr("Routine")
                selection.key.startsWith("media:") -> {
                    val media = canvas.mediaNodes.firstOrNull { it.id == selection.key.substringAfter(':') }
                    tr(if (media?.kind == "video") "Clip step" else "Picture step")
                }
                else -> tr("Bot")
            }
        })
        Column(Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(20.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
            when (selection) {
                is FlowSelection.OfLink -> canvas.links.firstOrNull { it.id == selection.id }?.let { link ->
                    LinkPanel(state, owner, link, bots[link.toBotId], name, online, busy, store, close)
                } ?: Text(tr("This link is gone."), style = Type.footnote, color = Palette.secondary)
                is FlowSelection.OfNode -> {
                    val id = selection.key.substringAfter(':')
                    when {
                        selection.key.startsWith("routine:") -> canvas.routines.firstOrNull { it.routine.id == id }?.let { card ->
                            RoutinePanel(owner, canvas, card, name, online, busy, store, reselect)
                        }
                        selection.key.startsWith("media:") -> canvas.mediaNodes.firstOrNull { it.id == id }?.let { media ->
                            MediaPanel(owner, canvas, media, targets, online, busy, store, reselect, close)
                        } ?: Text(tr("This step is gone."), style = Type.footnote, color = Palette.secondary)
                        else -> BotPanel(owner, canvas, bots[id], id, targets, name, online, busy, store, note, reselect, close)
                    }
                }
            }
        }
    }
}

@Composable private fun LinkPanel(state: AppState, owner: Bot, link: FlowLink, target: Bot?, name: (String) -> String,
    online: Boolean, busy: Boolean, store: AppStore, close: () -> Unit) {
    var text by remember(link.id) { mutableStateOf(link.instruction) }
    LaunchedEffect(link.instruction) { text = link.instruction }
    // Saved with the button, like the instruction; the step keeps them for its one turn.
    var model by remember(link.id) { mutableStateOf(link.model) }
    LaunchedEffect(link.model) { model = link.model }
    var options by remember(link.id) { mutableStateOf(link.modelOptions) }
    LaunchedEffect(link.modelOptions) { options = link.modelOptions }
    Text("${name(link.fromBotId)} → ${name(link.toBotId)}", style = Type.headline, color = Palette.text)
    Text(tr("What the next bot is asked to do with the reply. It can be empty."), style = Type.caption, color = Palette.secondary)
    Field("Instruction") { FieldBox(text, { text = it.take(8000) }, "e.g. Summarize it in three lines", 3, 8, fill = Palette.bubbleUser) }
    Field("Model", "The model this step runs on. Default is the bot's own.") {
        androidx.compose.runtime.key(state.generation, state.computer?.id, target?.backend) {
            AgentModelPicker(state, { backend -> store.agentModels(backend, target?.account) }, target?.backend.orEmpty(),
                model, options, false, target, { chosen, next -> model = chosen; options = next }, null)
        }
    }
    val changed = text.trim() != link.instruction || model != link.model || options != link.modelOptions
    PrimaryButton(tr("Save"), Modifier.fillMaxWidth(), enabled = online && !busy && changed) {
        store.updateFlowLink(owner.id, link.id, text.trim(), model, options)
    }
    SecondaryButton(tr("Disconnect"), Modifier.fillMaxWidth(), enabled = online && !busy) { store.disconnectFlow(owner.id, link.id); close() }
}

@Composable private fun RoutinePanel(owner: Bot, canvas: FlowCanvas, card: FlowRoutineCard, name: (String) -> String, online: Boolean,
    busy: Boolean, store: AppStore, reselect: (FlowSelection) -> Unit) {
    val routine = card.routine
    val linked = !RoutineFlow.isUnconnected(canvas, routine.id)
    Text(routine.name, style = Type.headline, color = Palette.text)
    Text(routine.triggerDescriptions.joinToString(" · "), style = Type.footnote, color = Palette.secondary)
    Row(verticalAlignment = Alignment.CenterVertically) {
        Text(tr(if (routine.enabled) "Active" else "Paused"), Modifier.weight(1f), style = Type.footnote, color = Palette.text)
        CodyncSwitch(routine.enabled, { store.setFlowRoutineEnabled(owner.id, routine.id, it) },
            Modifier.width(34.dp).semantics { contentDescription = routine.name }, enabled = online && !busy) {}
    }
    routine.nextRunAt?.let { Text("${tr("Next run")}: ${java.text.DateFormat.getDateTimeInstance().format(java.util.Date(it))}", style = Type.caption, color = Palette.secondary) }
    Text(card.recentRuns.firstOrNull()?.let { "${tr("Last run")}: ${tr(it.label)}" } ?: tr("No runs yet"), style = Type.caption, color = Palette.secondary)
    if (!linked) Text(tr("Not connected"), style = Type.caption, color = Palette.secondary)
    PrimaryButton(tr("Test the flow"), Modifier.fillMaxWidth(), enabled = online && !busy && linked) { store.testFlow(owner.id, routine.id) }
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        SecondaryButton(tr("Add a picture step"), Modifier.weight(1f), enabled = online && !busy) {
            store.addFlowMedia(owner.id, routine.id, "image") { media -> reselect(FlowSelection.OfNode(RoutineFlow.mediaKey(media.id))) }
        }
        SecondaryButton(tr("Add a clip step"), Modifier.weight(1f), enabled = online && !busy) {
            store.addFlowMedia(owner.id, routine.id, "video") { media -> reselect(FlowSelection.OfNode(RoutineFlow.mediaKey(media.id))) }
        }
    }
    if (card.steps.isNotEmpty()) {
        Text(tr("Latest run"), style = Type.caption, color = Palette.secondary)
        for (step in card.steps) Column(Modifier.fillMaxWidth().clip(RoundedRectangle(8.dp)).background(Palette.text.copy(alpha = 0.05f)).padding(8.dp),
            verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(name(step.botId), Modifier.weight(1f), style = Type.footnote, color = Palette.text, maxLines = 1, overflow = TextOverflow.Ellipsis)
                StatusChip(step.status)
            }
            step.error?.let { Text(it, style = Type.caption, color = Palette.danger, maxLines = 3, overflow = TextOverflow.Ellipsis) }
            step.output?.let { Text(it, style = Type.caption, color = Palette.secondary, maxLines = 3, overflow = TextOverflow.Ellipsis) }
        }
    }
    if (card.recentRuns.isNotEmpty()) {
        Text(tr("Run history"), style = Type.caption, color = Palette.secondary)
        for (run in card.recentRuns) Row(Modifier.fillMaxWidth()) {
            Text(tr(run.label), Modifier.weight(1f), style = Type.caption, color = Palette.text)
            Text(java.text.DateFormat.getDateTimeInstance().format(java.util.Date(run.createdAt)), style = Type.caption, color = Palette.secondary)
        }
    }
}

@Composable private fun BotPanel(owner: Bot, canvas: FlowCanvas, bot: Bot?, id: String, targets: (String) -> List<FlowTarget>,
    name: (String) -> String, online: Boolean, busy: Boolean, store: AppStore, note: (String?) -> Unit,
    reselect: (FlowSelection) -> Unit, close: () -> Unit) {
    val step = RoutineFlow.latestStep(canvas, id)
    Text(name(id), style = Type.headline, color = Palette.text)
    Text(bot?.model ?: bot?.backend.orEmpty(), style = Type.footnote, color = Palette.secondary)
    StatusChip(step?.status)
    step?.error?.let { Text(it, style = Type.caption, color = Palette.danger, maxLines = 4, overflow = TextOverflow.Ellipsis) }
    // Steps and bots this one may hand a reply to: each with the routines of this canvas that allow it.
    val options = targets(id).mapNotNull { target ->
        val routines = canvas.routines.filter { RoutineFlow.connectProblem(canvas, it.routine.id, id, target.id) == null }
        if (routines.isEmpty()) null else target to routines
    }
    fun connect(routine: String, to: String) {
        note(null)
        store.connectFlow(owner.id, routine, id, to) { link -> reselect(FlowSelection.OfLink(link.id)) }
    }
    DropdownMenu({ options.map { (target, routines) ->
        if (routines.size == 1) MenuItem(target.name) { connect(routines.single().routine.id, target.id) }
        else MenuItem(target.name, children = routines.map { card -> MenuItem(card.routine.name) { connect(card.routine.id, target.id) } })
    } }, enabled = online && !busy && options.isNotEmpty(), label = tr("Hand its reply on to…")) {
        Text(tr(if (options.isEmpty()) "No step to hand on to" else "Hand its reply on to…"),
            Modifier.fillMaxWidth().clip(RoundedRectangle(12.dp)).background(Palette.bubbleUser).padding(14.dp), style = Type.footnote, color = Palette.text)
    }
    val removable = id in canvas.placedBotIds && canvas.links.none { it.fromBotId == id || it.toBotId == id }
    if (removable) SecondaryButton(tr("Remove from the canvas"), Modifier.fillMaxWidth(), enabled = online && !busy) {
        store.removeFlowNode(owner.id, RoutineFlow.botKey(id)); close()
    }
}

/** The draft of a step's settings, with the kind its model makes. */
private fun FlowMedia.choice() = MediaChoice(kind, model, aspect, duration, resolution, transparent, voice)

/**
 * A picture or a clip step: its description, the model that makes the file and the choices that model
 * offers, plus what it may hand the file on to (host/src/media).
 * Ported from OpenBot 0.70.0 `WorkflowImageNode.tsx`'s inspector.
 */
@Composable private fun MediaPanel(owner: Bot, canvas: FlowCanvas, media: FlowMedia, targets: (String) -> List<FlowTarget>,
    online: Boolean, busy: Boolean, store: AppStore, reselect: (FlowSelection) -> Unit, close: () -> Unit) {
    val nodeKey = RoutineFlow.mediaKey(media.id)
    val step = canvas.routines.firstOrNull { it.routine.id == media.routineId }?.step(nodeKey)
    val kind = tr(if (media.kind == "video") "clip" else "picture")
    val kept = "${media.model}|${media.prompt}|${media.aspect}|${media.duration}|${media.resolution}|${media.transparent}|${media.voice}"
    var prompt by remember(media.id) { mutableStateOf(media.prompt) }
    var choice by remember(media.id) { mutableStateOf(media.choice()) }
    // The host's copy changed (a save, another device): show it; an edit in the box is kept otherwise.
    LaunchedEffect(kept) { prompt = media.prompt; choice = media.choice() }
    var answer by remember(media.id) { mutableStateOf<MediaAnswer?>(null) }
    var failed by remember(media.id) { mutableStateOf(false) }
    LaunchedEffect(media.id) {
        answer = null
        failed = false
        try {
            answer = store.mediaModels(owner.id)
        } catch (cancelled: kotlinx.coroutines.CancellationException) {
            throw cancelled
        } catch (_: Exception) {
            failed = true
        }
    }
    val models = answer.modelsOf(media.kind)
    val model = models.firstOrNull { it.id == choice.model }
    val current = media.choice()
    val changed = prompt != media.prompt || choice.model != current.model || choice.aspect != current.aspect ||
        choice.duration != current.duration || choice.resolution != current.resolution ||
        choice.transparent != current.transparent || choice.voice != current.voice
    Text(tr(if (media.kind == "video") "Clip step" else "Picture step"), style = Type.headline, color = Palette.text)
    StatusChip(step?.status)
    Text(tr("The step makes a {kind} from this description; whatever the step before hands on is added to it.").replace("{kind}", kind),
        style = Type.caption, color = Palette.secondary)
    Field("What to make") {
        FieldBox(prompt, { prompt = it.take(8000) }, tr("Describe the {kind} to make").replace("{kind}", kind), 3, 8, fill = Palette.bubbleUser)
    }
    Field("Model") {
        when {
            failed -> Text(tr("This computer's host is not answering."), style = Type.body, color = Palette.warning)
            answer == null -> Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                Spinner()
                Text(tr("Checking what this bot can make…"), style = Type.body, color = Palette.secondary)
            }
            answer?.agent == null -> Text(tr("This bot’s agent has no image tool. Pick one that does, or use ComfyUI."),
                style = Type.body, color = Palette.secondary)
            models.isEmpty() -> Text(tr("This bot’s agent makes no {kind}.").replace("{kind}", kind), style = Type.body, color = Palette.warning)
            else -> Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                if (answer?.installed == false) Text(tr("This bot’s agent is not installed on this computer yet."),
                    style = Type.caption, color = Palette.warning)
                if (models.size > 1) {
                    DropdownMenu({ models.map { each -> MenuItem(each.name, selected = each.id == choice.model) { choice = choice.onModel(each) } } },
                        label = tr("Pick a model")) {
                        Text(choice.let { models.firstOrNull { each -> each.id == it.model }?.name ?: tr("Pick a model") },
                            Modifier.fillMaxWidth().clip(RoundedRectangle(12.dp)).background(Palette.bubbleUser).padding(14.dp),
                            style = Type.footnote, color = Palette.text)
                    }
                } else model?.let { Text(it.name, style = Type.body, color = Palette.secondary) }
            }
        }
    }
    val picked = model?.let { choice }
    if (picked != null && model != null) {
        Field("Aspect ratio") {
            TabChoice(picked.aspect, model.capabilities.aspects.map { it to it }, { choice = picked.copy(aspect = it) })
        }
        if (media.kind == "video" && model.capabilities.durations.isNotEmpty()) Field("Length") {
            TabChoice(picked.duration, model.capabilities.durations.map { it to "${it}s" }, { choice = picked.copy(duration = it) })
        }
        if (media.kind == "video" && model.capabilities.resolutions.isNotEmpty()) Field("Resolution") {
            TabChoice(picked.resolution, model.capabilities.resolutions.map { it to it }, { choice = picked.copy(resolution = it) })
        }
        val voices = model.capabilities.voices
        if (media.kind == "video" && voices != null) Field("Voice") {
            TabChoice(picked.voice.orEmpty(), listOf("" to tr("None")) + voices.map { it.id to it.name }, {
                choice = picked.copy(voice = it.ifEmpty { null })
            })
        }
        if (media.kind == "image" && model.capabilities.transparent) {
            SwitchRow("Transparent background", isOn = picked.transparent, onChange = { choice = picked.copy(transparent = it) },
                enabled = online && !busy)
        }
    }
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        PrimaryButton(tr("Save"), Modifier.weight(1f), enabled = online && !busy && changed) {
            store.updateFlowMedia(owner.id, media.id, choice, prompt.trim())
        }
        SecondaryButton(tr("Remove the step"), Modifier.weight(1f), enabled = online && !busy) {
            store.removeFlowMedia(owner.id, media.id); close()
        }
    }
    // Steps and bots this one may hand its file to: each with the routines of this canvas that allow it.
    val options = targets(nodeKey).mapNotNull { target ->
        val routines = canvas.routines.filter { RoutineFlow.connectProblem(canvas, it.routine.id, nodeKey, target.id) == null }
        if (routines.isEmpty()) null else target to routines
    }
    DropdownMenu({ options.map { (target, routines) ->
        if (routines.size == 1) MenuItem(target.name) {
            store.connectFlow(owner.id, routines.single().routine.id, nodeKey, target.id) { link -> reselect(FlowSelection.OfLink(link.id)) }
        } else MenuItem(target.name, children = routines.map { card ->
            MenuItem(card.routine.name) {
                store.connectFlow(owner.id, card.routine.id, nodeKey, target.id) { link -> reselect(FlowSelection.OfLink(link.id)) }
            }
        })
    } }, enabled = online && !busy && options.isNotEmpty(), label = tr("Hand its file on to…")) {
        Text(tr(if (options.isEmpty()) "No step to hand on to" else "Hand its file on to…"),
            Modifier.fillMaxWidth().clip(RoundedRectangle(12.dp)).background(Palette.bubbleUser).padding(14.dp), style = Type.footnote, color = Palette.text)
    }
    step?.error?.let { Text(it, style = Type.caption, color = Palette.danger, maxLines = 4, overflow = TextOverflow.Ellipsis) }
    step?.output?.let { Text(it, style = Type.caption, color = Palette.secondary, maxLines = 4, overflow = TextOverflow.Ellipsis) }
}

