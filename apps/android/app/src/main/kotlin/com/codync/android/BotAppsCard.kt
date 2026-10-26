package com.codync.android

import androidx.compose.animation.animateContentSize
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.codync.android.core.BotApp
import com.codync.android.core.LinkState
import com.codync.android.core.MessageBookmark
import com.codync.android.core.OpenItem
import com.codync.android.core.collectDeliverables
import com.codync.android.core.collectPendingDecisions
import com.codync.android.design.*
import com.kyant.shapes.Capsule
import com.kyant.shapes.RoundedRectangle
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch

/**
 * Inspector card for a bot's sandboxed mini-apps (Ideas 1–7, 47, 48) and tracked open items (Ideas 8, 9).
 */
@Composable
internal fun BotAppsCard(botId: String, state: AppState, store: AppStore) {
    val scope = rememberCoroutineScope()
    val online = state.actualConnection is LinkState.Ready
    var apps by remember(botId, state.generation) { mutableStateOf<List<BotApp>>(emptyList()) }
    var openItems by remember(botId, state.generation) { mutableStateOf<List<OpenItem>>(emptyList()) }
    var bookmarks by remember(botId, state.generation) { mutableStateOf<List<MessageBookmark>>(emptyList()) }
    var loading by remember(botId) { mutableStateOf(true) }
    var error by remember(botId) { mutableStateOf<String?>(null) }
    var activeApp by remember { mutableStateOf<BotApp?>(null) }
    var editingNoteId by remember(botId) { mutableStateOf<String?>(null) }
    var noteDraft by remember(botId) { mutableStateOf("") }
    var savingNote by remember(botId) { mutableStateOf(false) }

    val entries = state.entries.filter { it.botId == botId }
    val pendingDecisions = remember(entries) { collectPendingDecisions(entries) }
    val deliverables = remember(entries) { collectDeliverables(entries) }

    suspend fun reload() {
        loading = true
        error = null
        try {
            apps = store.listBotApps(botId)
            openItems = store.listOpenItems(botId)
            bookmarks = store.listBookmarks(botId)
            val current = activeApp
            if (current != null) activeApp = apps.firstOrNull { it.slug == current.slug } ?: current
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (failure: Exception) {
            error = failure.message ?: "Could not load bot apps and saved items."
        } finally {
            loading = false
        }
    }

    suspend fun change(work: suspend () -> Unit): Boolean {
        try {
            work()
            reload()
            return error == null
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (failure: Exception) {
            error = failure.message ?: "Could not update this item."
            return false
        }
    }

    LaunchedEffect(botId, online, state.generation) {
        if (online) reload() else {
            loading = false
            error = "Connect this computer to load mini-apps and saved items."
        }
    }

    CardSection("Mini-Apps") {
        error?.let { message -> row {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(message, style = Type.footnote, color = Palette.danger)
                SecondaryButton("Retry loading", enabled = !loading) { scope.launch { reload() } }
            }
        } }
        if (loading && apps.isEmpty()) {
            row { Box(Modifier.fillMaxWidth().padding(vertical = 8.dp), contentAlignment = Alignment.Center) { Spinner() } }
        } else if (apps.isEmpty() && error == null) {
            row { Text(tr("No mini-apps yet. Ask the bot to build an interactive dashboard, tracker, or tool."), style = Type.body, color = Palette.secondary) }
        } else {
            for (app in apps) {
                row {
                    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                        Symbol("macwindow", size = 18.dp, tint = Palette.accent)
                        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                                Text(app.title, style = Type.body.semibold, color = Palette.text, maxLines = 1, overflow = TextOverflow.Ellipsis)
                                Text("v${app.version}", Modifier.clip(Capsule()).background(Palette.surface).padding(horizontal = 6.dp, vertical = 2.dp),
                                    style = Type.caption2.semibold, color = Palette.secondary)
                            }
                            app.description?.takeIf { it.isNotBlank() }?.let { description ->
                                Text(description, style = Type.caption, color = Palette.secondary, maxLines = 1, overflow = TextOverflow.Ellipsis)
                            }
                        }
                        SecondaryButton("Open") { activeApp = app }
                        IconButton("Delete app", "trash") {
                            scope.launch {
                                change { store.deleteBotApp(botId, app.slug) }
                            }
                        }
                    }
                }
            }
        }
    }

    Spacer(Modifier.height(16.dp))

    CardSection("Open Items (${openItems.size})") {
        row {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                Text(tr("Active tracked items"), style = Type.caption, color = Palette.secondary)
                SpinningCounter(value = openItems.size)
            }
        }
        if (openItems.isEmpty() && error == null && !loading) {
            row { Text(tr("No open items tracked right now."), style = Type.body, color = Palette.secondary) }
        } else {
            for (item in openItems) {
                row {
                    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                        Symbol("checklist", size = 16.dp, tint = Palette.secondary)
                        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                            Text(item.title, style = Type.body.medium, color = Palette.text, maxLines = 1, overflow = TextOverflow.Ellipsis)
                            if (item.note.isNotBlank()) {
                                Text(item.note, style = Type.caption, color = Palette.secondary, maxLines = 2, overflow = TextOverflow.Ellipsis)
                            }
                        }
                        IconButton("Mark resolved", "checkmark.circle") {
                            scope.launch {
                                change { store.dismissOpenItem(botId, item.id) }
                            }
                        }
                    }
                }
            }
        }
    }

    Spacer(Modifier.height(16.dp))

    CardSection("Pending Decisions (${pendingDecisions.size})") {
        if (pendingDecisions.isEmpty()) {
            row { Text(tr("No pending approvals or incidents."), style = Type.body, color = Palette.secondary) }
        } else {
            for (dec in pendingDecisions) {
                row {
                    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                        Symbol("exclamationmark.triangle.fill", size = 16.dp, tint = Palette.warning)
                        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                            Text(dec.title, style = Type.body.medium, color = Palette.text, maxLines = 1, overflow = TextOverflow.Ellipsis)
                            if (dec.detail.isNotBlank()) {
                                Text(dec.detail, style = Type.caption, color = Palette.secondary, maxLines = 2, overflow = TextOverflow.Ellipsis)
                            }
                        }
                    }
                }
            }
        }
    }

    Spacer(Modifier.height(16.dp))

    CardSection("Deliverables (${deliverables.size})") {
        if (deliverables.isEmpty()) {
            row { Text(tr("No files shared in this conversation yet."), style = Type.body, color = Palette.secondary) }
        } else {
            for (file in deliverables) {
                row {
                    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                        Symbol("doc.fill", size = 16.dp, tint = Palette.accent)
                        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                            Text(file.name, style = Type.body.medium, color = Palette.text, maxLines = 1, overflow = TextOverflow.Ellipsis)
                            Text("${maxOf(1L, file.size / 1024L)} KB · SHA-256 ${file.sha256.take(8)}", style = Type.caption, color = Palette.secondary)
                        }
                    }
                }
            }
        }
    }

    Spacer(Modifier.height(16.dp))

    CardSection("Bookmarks & Notes (${bookmarks.size})", modifier = Modifier.animateContentSize(Motion.layoutSize)) {
        if (bookmarks.isEmpty() && error == null && !loading) {
            row { Text(tr("Long-press any message in the chat to bookmark it here."), style = Type.body, color = Palette.secondary) }
        } else {
            for (bm in bookmarks) {
                row {
                    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                        Symbol("bookmark.fill", size = 15.dp, tint = Palette.accent)
                        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                            Text(bm.preview.ifBlank { "(Message)" }, style = Type.body.medium, color = Palette.text, maxLines = 1, overflow = TextOverflow.Ellipsis)
                            if (bm.note.isNotBlank() && editingNoteId != bm.entryId) {
                                Text(bm.note, style = Type.caption, color = Palette.secondary, maxLines = 2, overflow = TextOverflow.Ellipsis)
                            }
                        }
                        IconButton("Edit note", "pencil", enabled = !savingNote) {
                            editingNoteId = bm.entryId
                            noteDraft = bm.note
                            error = null
                        }
                        IconButton("Remove bookmark", "trash", enabled = !savingNote) {
                            scope.launch {
                                change { store.deleteBookmark(botId, bm.entryId) }
                            }
                        }
                    }
                }
                if (editingNoteId == bm.entryId) row {
                    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        PlainTextField(noteDraft, { noteDraft = it }, Modifier.fillMaxWidth(),
                            prompt = "Add a personal note…", minLines = 2, maxLines = 5, enabled = !savingNote)
                        error?.let { Text(it, style = Type.footnote, color = Palette.danger) }
                        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                            SecondaryButton(if (savingNote) "Saving…" else "Save note", enabled = !savingNote) {
                                savingNote = true
                                scope.launch {
                                    try {
                                        if (change { store.saveBookmark(botId, bm.entryId, bm.preview, noteDraft, bm.author, bm.threadId) }) {
                                            editingNoteId = null
                                            noteDraft = ""
                                        }
                                    } finally {
                                        savingNote = false
                                    }
                                }
                            }
                            SecondaryButton("Cancel", enabled = !savingNote) { editingNoteId = null; noteDraft = "" }
                        }
                    }
                }
            }
        }
    }

    CodyncSheet(activeApp, { activeApp = null }) { app ->
        BotAppSheet(botId = botId, app = app, store = store, onChanged = { scope.launch { reload() } })
    }
}
