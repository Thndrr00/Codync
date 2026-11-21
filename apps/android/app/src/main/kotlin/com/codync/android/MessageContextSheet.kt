package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.codync.android.core.Entry
import com.codync.android.design.*
import kotlinx.coroutines.CancellationException

/** The message a quote or search hit points at, with its neighbours from the host's context. */
internal data class MessageContextRequest(val botId: String, val entryId: String, val threadId: String?)

/**
 * One message and the span around it, read through `messageContext` — the same host call the
 * desktop's search jump uses. The matching message is highlighted and scrolled to; Continue opens
 * its chat or thread. Ported from OpenBot 0.70.0's message jump.
 */
@Composable internal fun MessageContextSheet(store: AppStore, request: MessageContextRequest, dismiss: () -> Unit,
    continueInChat: (() -> Unit)? = null) {
    var entries by remember(request.entryId) { mutableStateOf<List<Entry>?>(null) }
    var error by remember(request.entryId) { mutableStateOf<String?>(null) }
    val reduced = reduceMotion()
    val list = rememberLazyListState()
    LaunchedEffect(request.botId, request.entryId) {
        entries = null
        error = null
        try {
            entries = store.messageContext(request.botId, request.entryId)
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (failure: Exception) {
            error = failure.message ?: "Couldn't load the message."
        }
    }
    val loaded = entries
    LaunchedEffect(loaded) {
        if (loaded == null) return@LaunchedEffect
        val at = loaded.indexOfFirst { it.id == request.entryId }
        if (at >= 0) {
            if (reduced) list.scrollToItem(at, Alignment.CenterVertically)
            else list.animateScrollToItem(at, Alignment.CenterVertically)
        }
    }
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader("Message context") {
            PrimaryButton("Continue in chat") {
                if (continueInChat != null) continueInChat() else store.openBot(request.botId, request.threadId)
                dismiss()
            }
        }
        error?.let { Text(it, Modifier.padding(horizontal = 20.dp, vertical = 8.dp), style = Type.caption, color = Palette.danger) }
        if (loaded == null && error == null) {
            Row(Modifier.padding(20.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Spinner(16.dp)
                Text(tr("Loading…"), style = Type.body, color = Palette.secondary)
            }
        }
        LazyColumn(Modifier.fillMaxSize(), list, verticalArrangement = Arrangement.spacedBy(12.dp),
            contentPadding = androidx.compose.foundation.layout.PaddingValues(16.dp)) {
            items(loaded.orEmpty(), key = { it.id }) { entry ->
                val matching = entry.id == request.entryId
                Column(
                    Modifier.fillMaxWidth().background(
                        if (matching) Palette.accent.copy(alpha = 0.12f) else Palette.surface,
                        com.kyant.shapes.RoundedRectangle(12.dp)).padding(12.dp)
                        .semantics { contentDescription = (if (matching) "Matching message, " else "") +
                            (if (entry.kind == "user") "You, " else "Agent, ") + entry.text },
                    verticalArrangement = Arrangement.spacedBy(4.dp),
                ) {
                    Text(if (matching) "Matching message" else if (entry.kind == "user") "You" else "Agent",
                        style = Type.caption, color = Palette.secondary)
                    Text(entry.text, style = Type.body, color = Palette.text)
                }
            }
        }
    }
}
