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
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.unit.dp
import com.codync.android.design.*
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch

/** RSS and Atom feeds this computer follows, and the newest headlines (iOS `WorkspaceNewsFeedsSection`). */
@Composable
internal fun NewsFeedsSection(store: AppStore, onError: (String?) -> Unit) {
    val scope = rememberCoroutineScope()
    val uri = LocalUriHandler.current
    var feeds by remember { mutableStateOf<List<NewsFeed>>(emptyList()) }
    var items by remember { mutableStateOf<List<NewsItem>>(emptyList()) }
    var name by remember { mutableStateOf("") }
    var url by remember { mutableStateOf("") }
    var only by remember { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }

    suspend fun load() {
        try {
            val list = store.newsFeeds()
            val news = store.newsItems()
            feeds = list
            items = news
            onError(null)
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (failure: Exception) {
            onError(failure.message)
        }
    }

    LaunchedEffect(Unit) { load() }

    fun add() {
        if (url.isBlank()) return
        busy = true
        scope.launch {
            try {
                store.addNewsFeed(name.trim().ifBlank { "Feed" }, url.trim())
                name = ""
                url = ""
                load()
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (failure: Exception) {
                onError(failure.message)
            } finally {
                busy = false
            }
        }
    }

    fun remove(feed: NewsFeed) {
        scope.launch {
            try {
                store.removeNewsFeed(feed.id)
                if (only == feed.name) only = ""
                load()
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (failure: Exception) {
                onError(failure.message)
            }
        }
    }

    CardSection(
        "News feeds",
        "Paste an RSS or Atom address. Codync reads it at most every ten minutes and never writes to the site; bots read the same items with news_feed.",
    ) {
        row {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                FieldBox(name, { name = it }, "Name (optional)")
                FieldBox(url, { url = it }, "https://example.com/feed.xml")
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                    SecondaryButton("Add", enabled = url.isNotBlank() && !busy) { add() }
                }
            }
        }
        for (feed in feeds) row {
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    Text(feed.name, style = Type.body, color = Palette.text)
                    Text(feed.url, style = Type.footnote, color = Palette.secondary, maxLines = 1)
                }
                LinkButton(if (only == feed.name) "All feeds" else "Only this") { only = if (only == feed.name) "" else feed.name }
                LinkButton("Remove") { remove(feed) }
            }
        }
        if (feeds.isEmpty() && !busy) row {
            Text("No news feeds yet.", style = Type.footnote, color = Palette.secondary)
        }

        row { Text(only.ifEmpty { "Newest items" }, style = Type.footnote, color = Palette.secondary) }
        val visible = if (only.isEmpty()) items else items.filter { it.feed == only }
        for (item in visible) row {
            Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(item.title, style = Type.body, color = Palette.text, modifier = Modifier.plainButton { item.url?.let(uri::openUri) })
                Text(
                    listOfNotNull(item.feed, newsWhen(item.published)).joinToString(" · "),
                    style = Type.footnote,
                    color = Palette.secondary,
                )
                if (item.summary.isNotEmpty()) Text(item.summary, style = Type.footnote, color = Palette.tertiary, maxLines = 3)
            }
        }
        if (visible.isEmpty() && !busy) row {
            Text("No items yet. Add a feed or refresh.", style = Type.footnote, color = Palette.secondary)
        }
        row {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                SecondaryButton(if (busy) "Reading…" else "Refresh", enabled = !busy) { scope.launch { load() } }
            }
        }
    }
}

private fun newsWhen(ms: Long?): String? {
    if (ms == null) return null
    return SimpleDateFormat("d MMM", Locale.getDefault()).format(Date(ms))
}
