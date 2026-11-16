package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.codync.android.core.Bot
import com.codync.android.design.DropdownMenu
import com.codync.android.design.MenuItem
import com.codync.android.design.Palette
import com.codync.android.design.Symbol
import com.codync.android.design.Type
import com.codync.android.design.plainButton
import com.codync.android.design.tr

/**
 * A channel's request tree over the composer, as iOS `GroupTasksCard` and the desktop `GroupTasks`: the request, the
 * members' assignments and their children, each with its state. Stop pauses a row and its branch; a stopped row offers
 * Continue, Reassign (another member) and Cancel.
 */
@Composable
fun GroupTasksCard(bot: Bot, state: AppState, store: AppStore, modifier: Modifier = Modifier) {
    val tasks = state.bots.firstOrNull { it.id == bot.id }?.tasks.orEmpty().filter { it.state != "cancelled" }
    if (bot.kind != "group" || tasks.isEmpty()) return
    val members = bot.members.mapNotNull { id -> state.bots.firstOrNull { it.id == id } }
    Column(
        modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp)
            .clip(RoundedCornerShape(18.dp)).background(Palette.surface).padding(10.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(5.dp)) {
            Symbol("person.2", size = 10.dp, tint = Palette.tertiary)
            Text(tr("Waiting for sub-tasks"), style = Type.caption2, color = Palette.tertiary)
        }
        Column(Modifier.heightIn(max = 190.dp).verticalScroll(rememberScrollState())) {
            for (task in tasks) {
                Row(
                    Modifier.fillMaxWidth().padding(top = 4.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(5.dp),
                ) {
                    Text(task.ownerName ?: tr("The lead"), style = Type.caption, color = Palette.text, maxLines = 1)
                    Text(stateLabel(task.state), style = Type.caption2, color = Palette.tertiary)
                    if (task.instruction.isNotEmpty()) {
                        Text(
                            task.instruction,
                            style = Type.caption2,
                            color = Palette.tertiary,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                            modifier = Modifier.weight(1f, fill = false),
                        )
                    }
                }
                Row(horizontalArrangement = Arrangement.spacedBy(10.dp), verticalAlignment = Alignment.CenterVertically) {
                    if (task.isStopped) {
                        TaskAction("play.fill", "Continue") {
                            store.groupTask(bot.id, task.id, "continue")
                        }
                        DropdownMenu(items = {
                            members.filter { it.id != task.ownerId }.map { member ->
                                MenuItem(member.name) { store.groupTask(bot.id, task.id, "reassign", member.id) }
                            }
                        }) {
                            Row(
                                Modifier.padding(vertical = 5.dp),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(5.dp),
                            ) {
                                Symbol("arrow.left.arrow.right", size = 11.dp, tint = Palette.secondary)
                                Text(tr("Reassign"), style = Type.caption, color = Palette.secondary)
                            }
                        }
                        TaskAction("xmark", "Cancel") {
                            store.groupTask(bot.id, task.id, "cancel")
                        }
                    } else if (task.state == "queued" || task.state == "running") {
                        TaskAction("stop.fill", "Stop") {
                            store.groupTask(bot.id, task.id, "stop")
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun TaskAction(symbol: String, label: String, action: () -> Unit) {
    Box(Modifier.plainButton(action = action)) {
        Row(
            Modifier.padding(vertical = 5.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(5.dp),
        ) {
            Symbol(symbol, size = 10.dp, tint = Palette.secondary)
            Text(tr(label), style = Type.caption, color = Palette.secondary)
        }
    }
}

@Composable
private fun stateLabel(state: String): String = when (state) {
    "queued" -> tr("waiting")
    "running" -> tr("working")
    "completed" -> tr("done")
    "failed" -> tr("failed")
    "paused" -> tr("stopped")
    else -> tr("cancelled")
}
