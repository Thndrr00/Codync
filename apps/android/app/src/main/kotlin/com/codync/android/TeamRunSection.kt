package com.codync.android

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.codync.android.core.Bot
import com.codync.android.design.*

/** A job for several bots, written here and sent to one bot that runs it with its pipeline or batch tool. */
@Composable internal fun TeamRunSection(bots: List<Bot>, online: Boolean, send: (botId: String, text: String) -> Unit) {
    var coordinator by remember { mutableStateOf("") }
    var draft by remember { mutableStateOf(TeamRunDraft()) }
    var sentTo by remember { mutableStateOf<String?>(null) }
    val lead = bots.firstOrNull { it.id == coordinator }?.id ?: bots.firstOrNull()?.id.orEmpty()
    val name = { id: String -> bots.firstOrNull { it.id == id }?.name ?: "Unknown bot" }
    val issue = teamRunIssue(draft, lead)
    val options = listOf("" to "Choose a bot") + bots.filter { it.id != lead }.map { it.id to it.name }
    val pipeline = draft.mode == TeamRunMode.Pipeline
    fun setStage(index: Int, change: (TeamRunStage) -> TeamRunStage) {
        sentTo = null
        draft = draft.copy(stages = draft.stages.mapIndexed { i, s -> if (i == index) change(s) else s })
    }

    CardSection("Team run", "Write the job here; the coordinator bot runs it with its pipeline or batch tool and answers in its chat. It uses the bots' own folders and permissions.") {
        row {
            ValueRow("Coordinator") {
                ChoicePicker(lead, bots.map { it.id to it.name }, { coordinator = it }, label = "Coordinator", enabled = bots.isNotEmpty())
            }
        }
        row {
            ValueRow("Stages run") {
                ChoicePicker(draft.mode, listOf(TeamRunMode.Pipeline to "One after another", TeamRunMode.Parallel to "At the same time"),
                    { draft = draft.copy(mode = it) }, label = "Stages run")
            }
        }
        if (pipeline) row {
            PlainTextField(draft.input, { draft = draft.copy(input = it) }, Modifier.fillMaxWidth(),
                prompt = "Optional: what stage 1 gets as {{previous}}", singleLine = true)
        }
        draft.stages.forEachIndexed { index, stage ->
            row {
                Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    Text(if (pipeline) "Stage ${index + 1}" else "Task ${index + 1}", Modifier.weight(1f), style = Type.body.semibold, color = Palette.text)
                    ChoicePicker(stage.botId, options, { id -> setStage(index) { it.copy(botId = id) } }, label = "Bot for stage ${index + 1}")
                    if (draft.stages.size > 1) {
                        Symbol("xmark", size = 16.dp, tint = Palette.secondary,
                            modifier = Modifier.plainButton { draft = draft.copy(stages = draft.stages.filterIndexed { i, _ -> i != index }) })
                    }
                }
            }
            row {
                PlainTextField(stage.goal, { goal -> setStage(index) { it.copy(goal = goal) } }, Modifier.fillMaxWidth(),
                    prompt = if (pipeline && index > 0) "What to do. Use {{previous}} for the last reply." else "What this bot should do",
                    minLines = 2, maxLines = 6)
            }
        }
        row {
            ActionRow("plus", if (pipeline) "Add stage" else "Add task", draft.stages.size < MAX_TEAM_RUN_STAGES) {
                draft = draft.copy(stages = draft.stages + TeamRunStage())
            }
        }
        row {
            ActionRow("paperplane", "Send to ${name(lead)}", online && issue == null) {
                send(lead, teamRunMessage(draft, name))
                sentTo = name(lead)
                draft = TeamRunDraft(mode = draft.mode)
            }
        }
        if (issue != null && draft.stages.any { it.botId.isNotEmpty() || it.goal.isNotEmpty() }) row {
            Text(issue, style = Type.footnote, color = Palette.secondary)
        }
        sentTo?.let { row { Text("Sent to $it. Follow it in its chat.", style = Type.footnote, color = Palette.secondary) } }
    }
}
