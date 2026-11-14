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
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
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
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.selected
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

/**
 * Asks this chat what it already knows: its messages and its pages, best match first, each with its
 * snippet and date. A message opens in its context, a page opens here. The screen renders the
 * host-backed model; request and account lifetimes stay outside the view.
 */
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
    val page = state.page
    if (page != null) {
        PageView(page.first, page.second, model::closePage)
        return
    }
    Column(Modifier.fillMaxSize()) {
        ModalHeader("Search this chat")
        SearchField(state.query, model::updateQuery, Modifier.padding(horizontal = 20.dp, vertical = 8.dp), prompt = "Words to look for")
        LazyColumn(Modifier.fillMaxSize(), contentPadding = PaddingValues(start = 20.dp, end = 20.dp, bottom = 24.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)) {
            item {
                Column(Modifier.animateContentSize(Motion.layoutSize), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                        TabChoice(state.side, ChatSearchSide.entries.map { it to it.title }, model::updateSide, Modifier.weight(1f))
                        SecondaryButton(action = model::toggleTools, label = {
                            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                                Symbol("hammer", size = 12.dp)
                                Text("Tool traces")
                            }
                        }, modifier = Modifier.semantics { selected = state.tools })
                    }
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
                            Text("All chats search is still available.", style = Type.caption, color = Palette.secondary)
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
                state.query.isBlank() -> item { Text("What was said here, and what its pages hold. Every word has to appear.", Modifier.padding(vertical = 12.dp), style = Type.body, color = Palette.tertiary) }
                state.phase != SearchPhase.Failed && state.hits.isEmpty() -> item { Text("Nothing in this chat matches those words.", Modifier.padding(vertical = 12.dp), style = Type.body, color = Palette.tertiary) }
                else -> {
                    items(state.hits, key = { "${it.kind}:${it.id}" }) { hit -> SearchHitRow(hit, bot.name) { model.open(hit, close) } }
                    if (state.opening != null) item { Text("Opening result…", style = Type.caption, color = Palette.secondary) }
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

@Composable private fun SearchHitRow(hit: ChatHit, botName: String, open: () -> Unit) {
    Row(Modifier.fillMaxWidth().clip(RoundedRectangle(14.dp)).background(Palette.surface)
        .clickable(role = Role.Button, onClick = open)
        .padding(horizontal = 14.dp, vertical = 12.dp), verticalAlignment = Alignment.Top, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                Symbol(if (hit.isPage) "doc.text" else if (hit.isMine) "person" else "bubble.left", size = 12.dp, tint = Palette.secondary)
                Text(headline(hit, botName), style = Type.caption, color = Palette.secondary, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
                Text(RelativeTime.day(hit.at), style = Type.caption, color = Palette.tertiary, maxLines = 1)
            }
            Text(marked(hit.snippet), style = Type.chatSubheadline, color = Palette.secondary, maxLines = 3, overflow = TextOverflow.Ellipsis)
        }
    }
}

private fun headline(hit: ChatHit, botName: String): String {
    if (hit.isPage) return "${hit.title ?: "Page"} · page"
    val who = if (hit.isMine) "You" else botName
    return if (hit.author != null) "$who (${hit.author})" else who
}

/** The snippet with its matches in bold: the host brackets them (`[word]`). */
private fun marked(snippet: String): AnnotatedString = buildAnnotatedString {
    var at = 0
    while (at < snippet.length) {
        val open = snippet.indexOf('[', at)
        val close = if (open < 0) -1 else snippet.indexOf(']', open + 1)
        if (open < 0 || close < 0) { append(snippet.substring(at)); break }
        if (open > at) append(snippet.substring(at, open))
        withStyle(SpanStyle(fontWeight = FontWeight.SemiBold, color = Palette.text)) { append(snippet.substring(open + 1, close)) }
        at = close + 1
    }
}

/** A page opens in place, before its library. */
@Composable private fun PageView(title: String, body: String, back: () -> Unit) {
    Column(Modifier.fillMaxSize()) {
        ModalHeader(if (title.isBlank()) "Page" else title)
        Column(Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(horizontal = 20.dp, vertical = 8.dp)) {
            MarkdownText(body)
        }
        Row(Modifier.fillMaxWidth().padding(20.dp)) {
            SecondaryButton("Back to results", action = back)
        }
    }
}
