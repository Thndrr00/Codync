package com.codync.android

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.codync.android.core.ConversationLink
import com.codync.android.core.ThreadInfo
import com.codync.android.design.*
import kotlinx.coroutines.launch

/**
 * The Codync thread a channel conversation shares (card K13): what is linked now, a way to link one and to take it
 * away. The host names a conversation by its own ids (a Telegram chat id, a Slack thread timestamp, a Discord
 * channel), so only a Discord channel is the one this page shows; the other two are typed in.
 */
@Composable internal fun ChannelLinksRow(provider: String, channel: String, botId: String, store: AppStore) {
    val scope = rememberCoroutineScope()
    var links by remember(provider, channel, botId) { mutableStateOf<List<ConversationLink>>(emptyList()) }
    var threads by remember(provider, channel, botId) { mutableStateOf<List<ThreadInfo>>(emptyList()) }
    var conversation by remember(provider, channel) { mutableStateOf("") }
    var choice by remember(provider, channel) { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(provider, channel, botId) {
        links = runCatching { store.conversationLinks(botId) }.getOrDefault(emptyList())
        threads = runCatching { store.channelThreads(botId) }.getOrDefault(emptyList())
    }
    fun run(work: suspend () -> List<ConversationLink>) = scope.launch {
        busy = true
        try {
            links = work()
            error = null
        } catch (failure: Exception) {
            error = failure.message
        } finally {
            busy = false
        }
    }
    val typed = conversation.trim()
    val target = when (provider) {
        "discord" -> channel to ""
        "slack" -> channel to typed
        else -> typed to ""
    }
    val mine = links.filter { it.source == provider && (provider == "telegram" || it.channel == channel) }
    val name = { root: String -> threads.firstOrNull { it.id == root }?.name ?: root }
    Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text(tr("Linked conversation"), style = Type.caption, color = Palette.tertiary)
        for (link in mine) {
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    Text(tr("Linked to {thread}").replace("{thread}", name(link.rootId)), style = Type.caption, color = Palette.text)
                    Text(if (link.thread.isEmpty()) link.channel else "${link.channel} · ${link.thread}",
                        style = Type.caption, color = Palette.tertiary)
                }
                IconButton(tr("Unlink"), "minus.circle", enabled = !busy) {
                    run { store.unlinkConversation(botId, provider, link.channel, link.thread) }
                }
            }
        }
        if (provider != "discord") {
            FieldBox(conversation, { conversation = it },
                "Its id in the channel: a Telegram chat id, a Slack thread timestamp", 1, 1, fill = Palette.bubbleUser,
                capitalize = false)
        }
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            ChoicePicker(choice, threads.map { it.id to it.name }, { choice = it }, Modifier.weight(1f), enabled = !busy)
            SecondaryButton(tr("Link a thread"), enabled = !busy && choice.isNotEmpty() && target.first.isNotEmpty()) {
                run { store.linkConversation(botId, provider, target.first, target.second, choice) }
            }
        }
        error?.let { Text(it, style = Type.caption, color = Palette.danger) }
    }
}
