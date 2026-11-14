package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.codync.android.core.Bot
import com.codync.android.core.PinnedMessage
import com.codync.android.core.Pinner
import com.codync.android.design.*
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

/**
 * The chat's pinned messages, newest first, across the chat and its threads. A pin opens its
 * thread; the agent's pins are taken back here like its own tool takes them back.
 */
@Composable internal fun PinsSheet(bot: Bot, state: AppState, store: AppStore, close: () -> Unit) {
    var pins by remember(bot.id) { mutableStateOf<List<PinnedMessage>?>(null) }
    var failed by remember(bot.id) { mutableStateOf(false) }
    LaunchedEffect(bot.id) {
        pins = runCatching { store.pinnedMessages(bot.id) }.getOrNull()
        failed = pins == null
    }
    val agent = bot.name.ifBlank { tr("The agent") }
    val you = tr("You")
    val agentName: (String) -> String = { name -> if (name == "user") you else bot.name }
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader(tr("Pinned messages"))
        Text(tr("Kept at hand in this chat. A message can be unpinned by whoever reads it here."),
            style = Type.caption, color = Palette.secondary, modifier = Modifier.padding(horizontal = 20.dp, vertical = 4.dp))
        Hairline()
        LazyColumn(Modifier.fillMaxSize(), contentPadding = PaddingValues(16.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            items(pins.orEmpty(), key = { it.messageId }) { pin ->
                PinRow(pin, agent, agentName,
                    open = {
                        if (pin.threadId != null) store.openBot(bot.id, pin.threadId)
                        close()
                    },
                    unpin = { store.command("pinMessage", buildJsonObject {
                        put("botId", bot.id); put("messageId", pin.messageId); put("pinned", false)
                    }, "pin:" + pin.messageId) })
            }
            if (pins == null && !failed) item("loading") {
                Box(Modifier.fillMaxWidth().padding(vertical = 16.dp), contentAlignment = Alignment.Center) { Spinner(16.dp) }
            }
            if (failed) item("failed") {
                Text(tr("Couldn't load the pins."), style = Type.body, color = Palette.secondary, modifier = Modifier.padding(vertical = 12.dp))
            }
            if (pins?.isEmpty() == true) item("empty") {
                Text(tr("Nothing is pinned here yet. Open a message's menu and pick Pin."),
                    style = Type.body, color = Palette.secondary, modifier = Modifier.padding(vertical = 12.dp))
            }
        }
    }
}

@Composable private fun PinRow(pin: PinnedMessage, agent: String, authorName: (String) -> String,
    open: () -> Unit, unpin: () -> Unit) {
    val who = if (pin.pinnedBy.kind == "agent") tr("{name} (agent)").replace("{name}", agent)
    else pin.pinnedBy.device ?: tr("You")
    Row(Modifier.fillMaxWidth().plainButton(action = open).padding(vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        Column(Modifier.weight(1f)) {
            pin.threadTitle?.let { Text(it, style = Type.caption, color = Palette.tertiary, maxLines = 1, overflow = TextOverflow.Ellipsis) }
            Text(pin.text.ifBlank { tr("A message without text") }, style = Type.body, color = Palette.text, maxLines = 3,
                overflow = TextOverflow.Ellipsis)
            Text(who + " · " + authorName(pin.author), style = Type.caption, color = Palette.secondary, maxLines = 1,
                overflow = TextOverflow.Ellipsis)
        }
        IconButton(tr("Unpin"), "pin.slash", action = unpin)
    }
}
