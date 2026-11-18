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
import com.codync.android.design.semibold
import com.codync.android.design.tr

/**
 * A channel's request tree over the composer, as iOS `GroupTasksCard` and the desktop `GroupTasks`: the request, the
 * members' assignments and their children, each with its state. Stop pauses a row and its branch; a stopped row offers
 * Continue, Reassign (another member) and Cancel.
 */
@Composable
fun GroupTasksCard(bot: Bot, state: AppState, store: AppStore, modifier: Modifier = Modifier) {
    // The roster's copy: the tree moves while the chat stays open.
    val group = state.bots.firstOrNull { it.id == bot.id } ?: bot
    if (group.kind != "group") return
    val tasks = group.tasks.filter { it.state != "cancelled" }
    if (tasks.isEmpty()) return
    val members = group.members.mapNotNull { id -> state.bots.firstOrNull { it.id == id } }
    Column(
        modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp)
            .clip(RoundedCornerShape(18.dp)).background(Palette.surface).padding(10.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(5.dp)) {
            Symbol("person.2", size = 12.dp, tint = Palette.tertiary)
            Text(tr("Waiting for sub-tasks"), style = Type.caption2, color = Palette.tertiary)
        }
        // A long tree scrolls inside the card, so the composer never leaves the screen.
        Column(Modifier.heightIn(max = 190.dp).verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(6.dp)) {
            for (task in tasks) {
                Row(
                    Modifier.fillMaxWidth().padding(top = 4.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(5.dp),
                ) {
                    Text(task.ownerName ?: tr("The lead"), style = Type.caption.semibold, color = Palette.text, maxLines = 1)
                    Text(taskStateLabel(task.state), style = Type.caption2, color = Palette.tertiary, maxLines = 1)
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
                        TaskAction("play.fill", "Continue") { store.groupTask(group.id, task.id, "continue") }
                        // A channel with nobody else to give it to offers no Reassign.
                        val others = members.filter { it.id != task.ownerId }
                        if (others.isNotEmpty()) TaskMenuAction("arrow.left.arrow.right", "Reassign", items = {
                            others.map { member -> MenuItem(member.name) { store.groupTask(group.id, task.id, "reassign", member.id) } }
                        })
                        TaskAction("xmark", "Cancel") { store.groupTask(group.id, task.id, "cancel") }
                    } else if (task.state == "queued" || task.state == "running") {
                        TaskAction("stop.fill", "Stop") { store.groupTask(group.id, task.id, "stop") }
                    }
                }
            }
        }
    }
}

/** One named action on a row. */
@Composable private fun TaskAction(symbol: String, label: String, action: () -> Unit) {
    Box(Modifier.plainButton(action = action).heightIn(min = 32.dp), contentAlignment = Alignment.Center) {
        TaskLabel(symbol, label)
    }
}

/** One named action that opens a choice instead of acting at once (Reassign: another member). */
@Composable private fun TaskMenuAction(symbol: String, label: String, items: () -> List<MenuItem>) {
    DropdownMenu(items = items, modifier = Modifier.heightIn(min = 32.dp), label = tr(label)) { TaskLabel(symbol, label) }
}

@Composable private fun TaskLabel(symbol: String, label: String) {
    Row(Modifier.padding(horizontal = 9.dp), verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(5.dp)) {
        Symbol(symbol, size = 12.dp, tint = Palette.secondary)
        Text(tr(label), style = Type.caption.semibold, color = Palette.secondary, maxLines = 1)
    }
}

@Composable private fun taskStateLabel(state: String): String = when (state) {
    "queued" -> tr("waiting")
    "running" -> tr("working")
    "completed" -> tr("done")
    "failed" -> tr("failed")
    "paused" -> tr("stopped")
    else -> tr("cancelled")
}
