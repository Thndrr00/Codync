package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.codync.android.core.Entry
import com.codync.android.design.*
import com.kyant.shapes.RoundedRectangle

/** The "Waiting for replies" card above a chat's (or thread's) composer: one row per teammate
 * the agent asked, with a Stop that drops the request from its queue. Stopped rows stay visible. */
@Composable internal fun WaitingRepliesCard(botId: String, state: AppState, store: AppStore, threadId: String? = null,
    entries: List<Entry>) {
    var hidden by remember { mutableStateOf(false) }
    val all = entries + state.stoppedRequests.values
    val rows = WaitingReplies.rows(all, botId, threadId, state.stoppedRequests.keys.toSet()) { id ->
        state.bots.firstOrNull { it.id == id }?.name ?: id
    }
    if (hidden || rows.isEmpty()) return
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp)
        .clip(RoundedRectangle(16.dp)).background(Palette.bubbleAgent).padding(12.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(tr("Waiting for replies"), style = Type.subheadline.semibold, color = Palette.text,
                modifier = Modifier.weight(1f))
            if (rows.all { it.stopped }) SecondaryButton(tr("Hide this list")) { hidden = true }
        }
        for (row in rows) {
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                Column(Modifier.weight(1f)) {
                    Text(row.name, style = Type.footnote.semibold, color = Palette.text, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    Text(row.request, style = Type.caption, color = Palette.secondary, maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
                Text(tr(if (row.stopped) "Stopped" else "Asked"), style = Type.caption2,
                    color = if (row.stopped) Palette.tertiary else Palette.accent)
                if (!row.stopped) IconButton(tr("Stop") + " " + row.name, "xmark") { store.stopRequest(row.entry) }
            }
        }
    }
}
