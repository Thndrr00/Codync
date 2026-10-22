package com.codync.android

import android.content.ClipData
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.ClipEntry
import androidx.compose.ui.platform.LocalClipboard
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.codync.android.core.CodeHighlight
import com.codync.android.core.DiffRows
import com.codync.android.core.DiffRows.Row as DiffRow
import com.codync.android.design.*
import com.kyant.shapes.RoundedRectangle
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** Blocks up to this long are highlighted in the composition; longer ones are tokenized off the main thread. */
private const val SYNC_HIGHLIGHT_CHARS = 1_500

/**
 * A fenced code block of a reply: the language, a per-block wrap switch and a copy button
 * above the code. Copy puts the literal text on the clipboard, spaces and line breaks intact.
 * Diff blocks render through [DiffBlock]; other languages are highlighted locally or stay plain.
 */
@Composable fun CodeBlock(code: String, language: String) {
    val diff = remember(code, language) { if (DiffRows.looksLikeDiff(language, code)) DiffRows.parse(code) else null }
    if (diff != null) { DiffBlock(diff, code, language); return }
    var wrap by remember(code) { mutableStateOf(false) }
    val highlighted = rememberHighlighted(code, language)
    CodeFrame(label = language.trim().ifEmpty { "Code" }, copyText = code, wrap = wrap, onWrap = { wrap = !wrap }) {
        Box(Modifier.fillMaxWidth().then(if (wrap) Modifier else Modifier.horizontalScroll(rememberScrollState()))) {
            Text(highlighted, Modifier.padding(10.dp), style = Type.chatFootnote.monospaced, color = Palette.text, softWrap = wrap)
        }
    }
}

/**
 * A unified diff with old and new line numbers where the hunk ranges allow, the `+`/`-` marks kept
 * as text so colour is never the only cue, and copy returning the original diff. Display only.
 */
@Composable private fun DiffBlock(rows: List<DiffRow>, original: String, language: String) {
    CodeFrame(label = language.trim().ifEmpty { "diff" }, copyText = original, wrap = null, onWrap = {}) {
        Box(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState())) {
            Column {
                for (row in rows) DiffLine(row)
            }
        }
    }
}

@Composable private fun DiffLine(row: DiffRow) {
    val style = Type.chatFootnote.monospaced
    val (tint, ink) = when (row) {
        is DiffRow.Added -> Palette.added.copy(alpha = 0.14f) to Palette.text
        is DiffRow.Removed -> Palette.removed.copy(alpha = 0.14f) to Palette.text
        is DiffRow.Hunk -> Palette.accent.copy(alpha = 0.10f) to Palette.secondary
        is DiffRow.Header, is DiffRow.Note -> Color.Transparent to Palette.secondary
        is DiffRow.Context -> Color.Transparent to Palette.text
    }
    val (old, new) = when (row) {
        is DiffRow.Context -> row.oldNumber to row.newNumber
        is DiffRow.Added -> null to row.newNumber
        is DiffRow.Removed -> row.oldNumber to null
        else -> null to null
    }
    val numbered = row is DiffRow.Context || row is DiffRow.Added || row is DiffRow.Removed
    Row(Modifier.background(tint).padding(horizontal = 10.dp), verticalAlignment = Alignment.Top) {
        if (numbered) {
            Text(old?.toString().orEmpty(), Modifier.width(34.dp), style = style, color = Palette.tertiary, maxLines = 1)
            Text(new?.toString().orEmpty(), Modifier.width(34.dp), style = style, color = Palette.tertiary, maxLines = 1)
        }
        Text(row.raw.ifEmpty { " " }, style = if (row is DiffRow.Note) style.copy(fontStyle = FontStyle.Italic) else style, color = ink, softWrap = false)
    }
}

/** The header (label, optional wrap switch, copy) over a rounded code surface. */
@Composable private fun CodeFrame(label: String, copyText: String, wrap: Boolean?, onWrap: () -> Unit, body: @Composable () -> Unit) {
    Column(Modifier.fillMaxWidth().clip(RoundedRectangle(8.dp)).background(Palette.codeBackground)) {
        Row(Modifier.fillMaxWidth().padding(start = 10.dp, end = 2.dp), verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            Text(label, Modifier.weight(1f), style = Type.caption.monospaced, color = Palette.secondary, maxLines = 1, overflow = TextOverflow.Ellipsis)
            if (wrap != null) WrapSwitch(wrap, onWrap)
            CopyButton(copyText)
        }
        body()
    }
}

@Composable private fun WrapSwitch(wrapped: Boolean, toggle: () -> Unit) {
    val (source, pressed) = rememberPress()
    Box(Modifier.pressScale(pressed).clip(RoundedRectangle(8.dp)).background(if (wrapped) Palette.text.copy(alpha = 0.12f) else Color.Transparent)
        .clickable(source, null, role = Role.Switch, onClick = toggle)
        .semantics { contentDescription = "Wrap lines"; stateDescription = if (wrapped) "On" else "Off" }
        .padding(horizontal = 8.dp, vertical = 8.dp), contentAlignment = Alignment.Center) {
        Text("Wrap", style = Type.caption, color = if (wrapped) Palette.text else Palette.secondary, maxLines = 1)
    }
}

@Composable private fun CopyButton(text: String) {
    val clipboard = LocalClipboard.current
    val scope = rememberCoroutineScope()
    var copied by remember(text) { mutableStateOf(false) }
    LaunchedEffect(copied) { if (copied) { delay(1_500); copied = false } }
    IconButton(if (copied) "Copied" else "Copy code", if (copied) "checkmark" else "doc.on.doc") {
        scope.launch { clipboard.setClipEntry(ClipEntry(ClipData.newPlainText("Code", text))); copied = true }
    }
}

/** The code with its highlight; long blocks show plain text first and gain colour when tokenized off the main thread. */
@Composable private fun rememberHighlighted(code: String, language: String): AnnotatedString {
    val kinds = CodeHighlightColors(Palette.accent, Palette.secondary, Palette.added)
    if (code.length <= SYNC_HIGHLIGHT_CHARS) return remember(code, language, kinds) { highlight(code, CodeHighlight.spans(code, language), kinds) }
    val spans by produceState(emptyList(), code, language) { value = withContext(Dispatchers.Default) { CodeHighlight.spans(code, language) } }
    return remember(code, spans, kinds) { highlight(code, spans, kinds) }
}

private data class CodeHighlightColors(val keyword: Color, val comment: Color, val literal: Color)

private fun highlight(code: String, spans: List<CodeHighlight.Span>, colors: CodeHighlightColors): AnnotatedString = buildAnnotatedString {
    append(code)
    for (span in spans) addStyle(when (span.kind) {
        CodeHighlight.Kind.Keyword -> SpanStyle(color = colors.keyword)
        CodeHighlight.Kind.Comment -> SpanStyle(color = colors.comment, fontStyle = FontStyle.Italic)
        CodeHighlight.Kind.Literal, CodeHighlight.Kind.Number -> SpanStyle(color = colors.literal)
    }, span.start, span.end)
}
