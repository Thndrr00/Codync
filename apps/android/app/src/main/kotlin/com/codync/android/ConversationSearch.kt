package com.codync.android

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateContentSize
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
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
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.codync.android.core.Bot
import com.codync.android.design.*
import com.kyant.shapes.RoundedRectangle

/** The screen renders the host-backed model; request and account lifetimes stay outside the view. */
@Composable internal fun ConversationSearch(bot: Bot, store: AppStore, close: () -> Unit) {
    val appState by store.state.collectAsStateWithLifecycle()
    val parentScope = rememberCoroutineScope()
    val model = remember(store, bot.id, appState.contextId, appState.generation, appState.computer?.id, appState.loading) {
        ConversationSearchModel(AppStoreConversationSearchHost(store), bot.id, parentScope)
    }
    DisposableEffect(model) {
        model.loadShortcuts()
        onDispose { model.dispose() }
    }
    val state = model.state
    Column(Modifier.fillMaxSize()) {
        ModalHeader("Search conversation")
        SearchField(state.query, model::updateQuery, Modifier.padding(horizontal = 20.dp, vertical = 8.dp), prompt = "Search messages")
        LazyColumn(Modifier.fillMaxSize(), contentPadding = androidx.compose.foundation.layout.PaddingValues(start = 20.dp, end = 20.dp, bottom = 24.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)) {
            item {
                Column(Modifier.animateContentSize(Motion.layoutSize), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    if (state.shortcutPhase == ShortcutPhase.Ready) {
                        SegmentedChoice(state.location, MessageSearchScope.entries.map { it to it.title }, model::updateScope)
                        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                            LinkButton("Saved searches (${state.shortcuts.size})", Modifier.weight(1f)
                                .semantics { stateDescription = if (state.showShortcuts) "Expanded" else "Collapsed" }, style = Type.caption,
                                action = model::toggleShortcuts)
                            SecondaryButton("Save search", enabled = state.query.isNotBlank() && !state.shortcutBusy, action = model::save)
                        }
                        AnimatedVisibility(state.showShortcuts, enter = fadeIn(Motion.fade()) + expandVertically(Motion.layoutSize),
                            exit = fadeOut(Motion.fade()) + shrinkVertically(Motion.layoutSize)) {
                            Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                                if (state.shortcuts.isEmpty()) Text("No saved searches yet.", style = Type.caption, color = Palette.secondary)
                                for (shortcut in state.shortcuts) SavedSearchRow(shortcut, !state.shortcutBusy,
                                    restore = { model.restore(shortcut) }, remove = { model.remove(shortcut) })
                            }
                        }
                    }
                    if (state.shortcutPhase == ShortcutPhase.Loading) Text("Loading saved searches…", style = Type.caption, color = Palette.secondary)
                    state.shortcutError?.let { message ->
                        Text(message, style = Type.caption, color = Palette.danger)
                        if (state.shortcutPhase == ShortcutPhase.Unavailable) {
                            Text("All messages search is still available.", style = Type.caption, color = Palette.secondary)
                            SecondaryButton("Retry saved searches", action = model::loadShortcuts)
                        }
                    }
                }
            }
            if (state.error != null) item {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(state.error, style = Type.body, color = Palette.danger)
                    if (state.phase == SearchPhase.Failed) SecondaryButton("Retry search", action = model::retry)
                }
            }
            when {
                state.phase == SearchPhase.Searching && state.hits.isEmpty() -> item { Box(Modifier.fillMaxWidth().padding(vertical = 24.dp), contentAlignment = Alignment.Center) { Spinner(16.dp) } }
                state.query.isBlank() -> item { Text("Search this conversation.", Modifier.padding(vertical = 12.dp), style = Type.body, color = Palette.tertiary) }
                state.phase != SearchPhase.Failed && state.hits.isEmpty() -> item { Text("No matches.", Modifier.padding(vertical = 12.dp), style = Type.body, color = Palette.tertiary) }
                else -> {
                    items(state.hits, key = { it.id }) { hit -> SearchHitRow(hit, state.query.trim()) { model.open(hit, close) } }
                    if (state.opening != null) item { Text("Opening result…", style = Type.caption, color = Palette.secondary) }
                    if (state.next != null) item {
                        Box(Modifier.fillMaxWidth().padding(vertical = 8.dp), contentAlignment = Alignment.Center) {
                            if (state.phase == SearchPhase.LoadingMore) Spinner(16.dp)
                            else SecondaryButton("More", action = model::more)
                        }
                    }
                }
            }
        }
    }
}

@Composable private fun SavedSearchRow(shortcut: SearchShortcut, enabled: Boolean, restore: () -> Unit, remove: () -> Unit) {
    Row(Modifier.fillMaxWidth().clip(RoundedRectangle(14.dp)).background(Palette.surface).padding(horizontal = 12.dp),
        verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        Column(Modifier.weight(1f).plainButton(enabled = enabled, action = restore).padding(vertical = 10.dp), verticalArrangement = Arrangement.spacedBy(3.dp)) {
            Text(shortcut.query, style = Type.body, color = Palette.text, maxLines = 2, overflow = TextOverflow.Ellipsis)
            Text(MessageSearchScope.entries.firstOrNull { it.wire == shortcut.scope }?.title ?: "Unsupported scope", style = Type.caption, color = Palette.secondary)
        }
        IconButton("Remove saved search ${shortcut.query}", "trash", enabled = enabled, action = remove)
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
