package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
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
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import com.codync.android.core.Bot
import com.codync.android.design.*
import com.kyant.shapes.RoundedRectangle
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/**
 * Search in a conversation (idea 30): query the host's `searchMessages`, show each hit's text and
 * where it lives, and jump to it (loading the span around it first). Changing the query cancels the
 * previous search; nothing here edits the Markdown or the mirror's history.
 */
@Composable internal fun ConversationSearch(bot: Bot, store: AppStore, close: () -> Unit) {
    var query by remember { mutableStateOf("") }
    var hits by remember { mutableStateOf<List<SearchHit>>(emptyList()) }
    var next by remember { mutableStateOf<Long?>(null) }
    var loading by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    LaunchedEffect(query) {
        val trimmed = query.trim()
        if (trimmed.isEmpty()) { hits = emptyList(); next = null; loading = false; error = null; return@LaunchedEffect }
        loading = true; error = null
        delay(300)
        try {
            val page = store.searchMessages(bot.id, trimmed)
            hits = page.hits; next = page.next
        } catch (e: CancellationException) { throw e }
        catch (e: Exception) { hits = emptyList(); next = null; error = e.message ?: "Search failed." }
        finally { loading = false }
    }

    fun openHit(hit: SearchHit) {
        close()
        store.openSearchHit(bot.id, hit.id, hit.threadId)
    }

    Column(Modifier.fillMaxSize()) {
        ModalHeader("Search conversation")
        SearchField(query, { query = it }, Modifier.padding(horizontal = 20.dp, vertical = 8.dp), prompt = "Search messages")
        LazyColumn(Modifier.fillMaxSize(), contentPadding = androidx.compose.foundation.layout.PaddingValues(start = 20.dp, end = 20.dp, bottom = 24.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp)) {
            when {
                loading && hits.isEmpty() -> item { Box(Modifier.fillMaxWidth().padding(vertical = 24.dp), contentAlignment = Alignment.Center) { Spinner(16.dp) } }
                error != null -> item { Text(error.orEmpty(), Modifier.padding(vertical = 12.dp), style = Type.body, color = Palette.danger) }
                query.isBlank() -> item { Text("Search this conversation.", Modifier.padding(vertical = 12.dp), style = Type.body, color = Palette.tertiary) }
                hits.isEmpty() -> item { Text("No matches.", Modifier.padding(vertical = 12.dp), style = Type.body, color = Palette.tertiary) }
                else -> {
                    items(hits, key = { it.id }) { hit -> SearchHitRow(hit, query.trim()) { openHit(hit) } }
                    if (next != null) item {
                        Box(Modifier.fillMaxWidth().padding(vertical = 8.dp), contentAlignment = Alignment.Center) {
                            Text("More", Modifier.clickable(role = Role.Button) {
                                scope.launch {
                                    try { val page = store.searchMessages(bot.id, query.trim(), next); hits = hits + page.hits; next = page.next }
                                    catch (e: CancellationException) { throw e }
                                    catch (_: Exception) {}
                                }
                            }.padding(horizontal = 12.dp, vertical = 8.dp), style = Type.body.medium, color = Palette.accent)
                        }
                    }
                }
            }
        }
    }
}

@Composable private fun SearchHitRow(hit: SearchHit, query: String, open: () -> Unit) {
    val accent = Palette.accent
    Row(Modifier.fillMaxWidth().clip(RoundedRectangle(14.dp)).background(Palette.surface)
        .clickable(role = Role.Button, onClick = open)
        .padding(horizontal = 14.dp, vertical = 12.dp), verticalAlignment = Alignment.Top, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(3.dp)) {
            Text(highlightMatch(hit.text.ifBlank { "…" }, query, accent), style = Type.chatSubheadline, color = Palette.text, maxLines = 2, overflow = TextOverflow.Ellipsis)
            Text(if (hit.threadId != null) "In a thread · ${RelativeTime.day(hit.createdAt)}" else RelativeTime.day(hit.createdAt),
                style = Type.caption2, color = Palette.tertiary, maxLines = 1)
        }
        if (hit.threadId != null) Symbol("arrowshape.turn.up.left", size = 14.dp, tint = Palette.secondary)
    }
}

/** The snippet with the query tinted (case-insensitive, as the host matches); the Markdown is untouched. */
private fun highlightMatch(text: String, query: String, accent: Color): AnnotatedString {
    if (query.isBlank()) return AnnotatedString(text)
    return buildAnnotatedString {
        var at = 0
        while (at < text.length) {
            val hit = text.indexOf(query, at, ignoreCase = true)
            if (hit < 0) { append(text.substring(at)); break }
            if (hit > at) append(text.substring(at, hit))
            withStyle(SpanStyle(color = accent, fontWeight = FontWeight.SemiBold)) { append(text.substring(hit, hit + query.length)) }
            at = hit + query.length
        }
    }
}
