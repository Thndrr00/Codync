package com.codync.android

import android.annotation.SuppressLint
import android.content.ClipData
import android.content.ClipboardManager
import android.webkit.WebView
import android.webkit.WebViewClient
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import com.codync.android.core.BotApp
import com.codync.android.core.LinkState
import com.codync.android.core.MessageBookmark
import com.codync.android.core.OpenItem
import com.codync.android.core.collectDeliverables
import com.codync.android.core.collectPendingDecisions
import com.codync.android.design.*
import com.kyant.shapes.Capsule
import com.kyant.shapes.RoundedRectangle
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
    var activeApp by remember { mutableStateOf<BotApp?>(null) }

    val entries = state.entries.filter { it.botId == botId }
    val pendingDecisions = remember(entries) { collectPendingDecisions(entries) }
    val deliverables = remember(entries) { collectDeliverables(entries) }

    suspend fun reload() {
        loading = true
        apps = runCatching { store.listBotApps(botId) }.getOrDefault(emptyList())
        openItems = runCatching { store.listOpenItems(botId) }.getOrDefault(emptyList())
        bookmarks = runCatching { store.listBookmarks(botId) }.getOrDefault(emptyList())
        val current = activeApp
        if (current != null) activeApp = apps.firstOrNull { it.slug == current.slug } ?: current
        loading = false
    }

    LaunchedEffect(botId, online, state.generation) {
        if (online) reload() else loading = false
    }

    CardSection("Mini-Apps") {
        if (loading && apps.isEmpty()) {
            row { Box(Modifier.fillMaxWidth().padding(vertical = 8.dp), contentAlignment = Alignment.Center) { Spinner() } }
        } else if (apps.isEmpty()) {
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
                                runCatching { store.deleteBotApp(botId, app.slug) }
                                reload()
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
        if (openItems.isEmpty()) {
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
                                runCatching { store.dismissOpenItem(botId, item.id) }
                                reload()
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

    CardSection("Bookmarks & Notes (${bookmarks.size})") {
        if (bookmarks.isEmpty()) {
            row { Text(tr("Long-press any message in the chat to bookmark it here."), style = Type.body, color = Palette.secondary) }
        } else {
            for (bm in bookmarks) {
                row {
                    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                        Symbol("bookmark.fill", size = 15.dp, tint = Palette.accent)
                        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                            Text(bm.preview.ifBlank { "(Message)" }, style = Type.body.medium, color = Palette.text, maxLines = 1, overflow = TextOverflow.Ellipsis)
                            if (bm.note.isNotBlank()) {
                                Text(bm.note, style = Type.caption, color = Palette.secondary, maxLines = 2, overflow = TextOverflow.Ellipsis)
                            }
                        }
                        IconButton("Remove bookmark", "trash") {
                            scope.launch {
                                runCatching { store.deleteBookmark(botId, bm.entryId) }
                                reload()
                            }
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

/** Sandboxed mini-app sheet with capability link sharing (`view` / `edit`). */
@SuppressLint("SetJavaScriptEnabled")
@Composable
internal fun BotAppSheet(botId: String, app: BotApp, store: AppStore, onChanged: () -> Unit = {}) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var shareMode by remember { mutableStateOf("view") }
    var shareStatus by remember { mutableStateOf<String?>(null) }

    Column(Modifier.fillMaxSize()) {
        ModalHeader("${app.title} · v${app.version}") {
            ChoicePicker(shareMode, listOf("view" to "View", "edit" to "Edit"), { shareMode = it }, label = "Share mode")
            IconButton("Share link", "square.and.arrow.up") {
                scope.launch {
                    val link = runCatching { store.createAppLink(botId, app.slug, shareMode, "Mobile $shareMode") }.getOrNull()
                    if (link != null) {
                        val url = app.url.ifBlank { "/apps/$botId/${app.slug}/${link.token}/" }
                        context.getSystemService(ClipboardManager::class.java)
                            ?.setPrimaryClip(ClipData.newPlainText("Mini-App Link", url))
                        shareStatus = "Copied $shareMode link: $url"
                        onChanged()
                    }
                }
            }
        }
        shareStatus?.let { msg ->
            Text(msg, Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 6.dp), style = Type.caption, color = Palette.accent)
        }
        if (app.url.isNotBlank()) {
            AndroidView(
                modifier = Modifier.fillMaxWidth().weight(1f).padding(horizontal = 12.dp, vertical = 8.dp).clip(RoundedRectangle(16.dp)),
                factory = { ctx ->
                    WebView(ctx).apply {
                        settings.javaScriptEnabled = true
                        settings.domStorageEnabled = false
                        settings.allowFileAccess = false
                        settings.allowContentAccess = false
                        webViewClient = WebViewClient()
                        loadUrl(app.url)
                    }
                },
                update = { view ->
                    if (view.url != app.url) view.loadUrl(app.url)
                },
            )
        } else {
            Box(Modifier.fillMaxWidth().weight(1f), contentAlignment = Alignment.Center) {
                Text("Mini-app ready (${app.entry} · v${app.version})", style = Type.body, color = Palette.secondary)
            }
        }
    }
}
