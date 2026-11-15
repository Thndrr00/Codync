package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.unit.dp
import com.codync.android.core.Mission
import com.codync.android.core.MissionStatus
import com.codync.android.core.MissionStep
import com.codync.android.core.MissionStepStatus
import com.codync.android.design.*

/**
 * A bot's missions (iOS `MissionsView`): the objective that outlives a turn, its steps and how far it got, with the
 * same actions as the desktop inspector and the terminal UI.
 */
@Composable fun MissionsCard(botId: String, state: AppState, store: AppStore) {
    val scope = rememberCoroutineScope()
    val model = remember(botId, state.generation) { MissionsModel(botId, store, scope, state.generation) }
    var composing by remember(botId) { mutableStateOf(false) }
    var abandoning by remember(botId) { mutableStateOf<Mission?>(null) }
    var reason by remember(botId) { mutableStateOf("") }
    LaunchedEffect(model) { model.watch() }
    CardSection(
        "Missions",
        footer = "A mission is a goal that outlives one turn: the bot records what is done and what is left, so the next turn resumes instead of guessing.",
        accessory = { IconButton("New mission", "plus", enabled = !model.busy) { composing = true } },
    ) {
        model.error?.let { error -> row { Text(error, style = Type.body, color = Palette.danger) } }
        if (model.loading && model.missions.isEmpty()) row { Text("Loading missions…", style = Type.body, color = Palette.secondary) }
        if (!model.loading && model.open.isEmpty()) row { Text("No open missions.", style = Type.body, color = Palette.secondary) }
        for (mission in model.open) row {
            MissionRow(mission, model) { asked ->
                reason = ""
                abandoning = asked
            }
        }
        if (model.finished.isNotEmpty()) row {
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Text("Finished", Modifier.weight(1f), style = Type.body.semibold, color = Palette.text)
                IconButton(
                    if (model.finishedShown) "Hide finished" else "Show finished",
                    if (model.finishedShown) "chevron.up" else "chevron.down",
                ) { model.toggleFinished() }
            }
        }
        if (model.finishedShown) {
            for (mission in model.finished) row { FinishedRow(mission, model) { asked -> reason = ""; abandoning = asked } }
        } else if (model.finished.isNotEmpty()) {
            row { Text("Finished missions are hidden.", style = Type.caption, color = Palette.secondary) }
        }
    }
    CodyncSheet(composing, { composing = false }) { NewMissionSheet(model) { composing = false } }
    CodyncDialog(
        "Abandon mission",
        abandoning != null,
        { abandoning = null },
        message = abandoning?.title,
        field = { PlainTextField(reason, { reason = it }, prompt = "Why it stopped", imeAction = androidx.compose.ui.text.input.ImeAction.Done) },
    ) {
        listOf(DialogAction("Abandon", destructive = true) {
            val target = abandoning
            if (target != null && reason.isNotBlank()) model.move(target, MissionStatus.ABANDONED, reason.trim())
        })
    }
}

/** The glyph of the next move: pause, resume, or back into the list of open ones. */
private fun moveIcon(status: MissionStatus): String = when (status) {
    MissionStatus.ACTIVE -> "pause"
    MissionStatus.DONE, MissionStatus.ABANDONED -> "arrow.uturn.backward"
    MissionStatus.PROPOSED, MissionStatus.WAITING -> "play"
}

@Composable private fun MissionRow(mission: Mission, model: MissionsModel, askAbandon: (Mission) -> Unit) {
    var checkpoint by remember(mission.id) { mutableStateOf(mission.checkpoint) }
    var draft by remember(mission.id) { mutableStateOf("") }
    LaunchedEffect(mission.checkpoint) { checkpoint = mission.checkpoint }
    Column(Modifier.fillMaxWidth().padding(vertical = 4.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            MissionPill(mission.status)
            Text(mission.title, Modifier.weight(1f), style = Type.body.semibold, color = Palette.text)
            mission.status.next?.let { next ->
                IconButton(next.nextTitle, moveIcon(mission.status), enabled = !model.busy) {
                    model.move(mission, next)
                }
            }
            IconButton("Finish", "checkmark.seal", enabled = !model.busy) { model.move(mission, MissionStatus.DONE) }
            IconButton("Abandon", "xmark.circle", enabled = !model.busy) { askAbandon(mission) }
        }
        if (mission.goal.isNotBlank() && mission.goal != mission.title) Text(mission.goal, style = Type.caption, color = Palette.secondary)
        mission.stopReason?.let { Text(it, style = Type.caption, color = Palette.secondary) }
        for (step in mission.steps) StepRow(mission, step, model)
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            PlainTextField(draft, { draft = it }, Modifier.weight(1f), prompt = "Add a step", singleLine = true,
                imeAction = androidx.compose.ui.text.input.ImeAction.Done,
                actions = androidx.compose.foundation.text.KeyboardActions(onDone = {
                    model.addStep(mission, draft)
                    draft = ""
                }))
            IconButton("Add step", "plus", enabled = !model.busy && draft.isNotBlank()) {
                model.addStep(mission, draft)
                draft = ""
            }
        }
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            PlainTextField(checkpoint, { checkpoint = it }, Modifier.weight(1f), prompt = "What is done and what is left", minLines = 2, maxLines = 4)
            IconButton("Save checkpoint", "square.and.arrow.down", enabled = !model.busy && checkpoint != mission.checkpoint) {
                model.checkpoint(mission, checkpoint)
            }
        }
    }
}

@Composable private fun StepRow(mission: Mission, step: MissionStep, model: MissionsModel) {
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(step.status.mark, Modifier.width(14.dp), style = Type.caption, color = Palette.secondary)
        Text(
            step.text, Modifier.weight(1f), style = Type.caption,
            color = if (step.status == MissionStepStatus.DONE || step.status == MissionStepStatus.SKIPPED) Palette.secondary else Palette.text,
            textDecoration = if (step.status == MissionStepStatus.DONE) TextDecoration.LineThrough else null,
        )
        if (step.status != MissionStepStatus.DONE) {
            IconButton("Start step", "play", enabled = !model.busy && step.status != MissionStepStatus.ACTIVE) {
                model.setStep(mission, step, MissionStepStatus.ACTIVE)
            }
            IconButton("Mark done", "checkmark.circle", enabled = !model.busy) { model.setStep(mission, step, MissionStepStatus.DONE) }
            IconButton("Skip step", "forward", enabled = !model.busy) { model.setStep(mission, step, MissionStepStatus.SKIPPED) }
        } else {
            IconButton("Reopen step", "arrow.uturn.backward", enabled = !model.busy) { model.setStep(mission, step, MissionStepStatus.PENDING) }
        }
    }
}

@Composable private fun FinishedRow(mission: Mission, model: MissionsModel, askAbandon: (Mission) -> Unit) {
    Column(Modifier.fillMaxWidth().padding(vertical = 4.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            MissionPill(mission.status)
            Text(mission.title, style = Type.body.semibold, color = Palette.text)
        }
        mission.stopReason?.let { Text(it, style = Type.caption, color = Palette.secondary) }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            SecondaryButton("Reopen", enabled = !model.busy) { model.move(mission, MissionStatus.ACTIVE) }
            SecondaryButton("Abandon", enabled = !model.busy) { askAbandon(mission) }
        }
    }
}

@Composable private fun MissionPill(status: MissionStatus) {
    val color = when (status) {
        MissionStatus.ACTIVE -> Palette.accent
        MissionStatus.WAITING -> Palette.warning
        MissionStatus.DONE -> Palette.added
        else -> Palette.secondary
    }
    Text(
        status.label,
        Modifier.background(color.copy(alpha = if (status == MissionStatus.PROPOSED || status == MissionStatus.ABANDONED) 0.10f else 0.16f), RoundedCornerShape(50))
            .padding(horizontal = 6.dp, vertical = 2.dp),
        style = Type.caption, color = color,
    )
}

/** The new-mission sheet: what to reach, why, and the steps somebody already knows. */
@Composable private fun NewMissionSheet(model: MissionsModel, onCreated: () -> Unit) {
    var title by remember { mutableStateOf("") }
    var goal by remember { mutableStateOf("") }
    var steps by remember { mutableStateOf("") }
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader("New mission") { if (model.busy) Spinner() }
        Column(Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(20.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
            PlainTextField(title, { title = it }, prompt = "What is the objective?", style = Type.body.semibold)
            PlainTextField(goal, { goal = it }, prompt = "Why, and what counts as done", minLines = 2, maxLines = 5)
            PlainTextField(steps, { steps = it }, prompt = "One step per line, if you know them", minLines = 3, maxLines = 10)
            model.error?.let { Text(it, style = Type.body, color = Palette.danger) }
            PrimaryButton("Create mission", Modifier.fillMaxWidth(), enabled = !model.busy && title.isNotBlank()) {
                model.create(title, goal, steps.lines())
                onCreated()
            }
        }
    }
}
