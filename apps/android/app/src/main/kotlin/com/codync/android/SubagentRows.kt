package com.codync.android

import androidx.compose.animation.animateContentSize
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangementimport androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.codync.android.core.Entry
import com.codync.android.core.EntrySubagent
import com.codync.android.design.*
import com.kyant.shapes.RoundedRectangle
import kotlinx.coroutines.delay

/** The time now, once a second while [running] (`iOS`'s TimelineView). */
@Composable private fun rememberNow(running: Boolean): Long {
    var now by remember { mutableStateOf(System.currentTimeMillis()) }
    LaunchedEffect(running) {
        now = System.currentTimeMillis()
        while (running) {
            delay(1_000)
            now = System.currentTimeMillis()
        }
    }
    return now
}

/** "12s" or "1m 05s": how long a subagent, or all of them, took. */
internal fun subagentDuration(ms: Long): String {
    val total = (ms / 1_000).coerceAtLeast(0)
    if (total < 60) return "${total}s"
    return "${total / 60}m " + (total % 60).toString().padStart(2, '0') + "s"
}

internal fun subagentDuration(subagents: List<EntrySubagent>, now: Long): String {
    if (subagents.isEmpty()) return "0s"
    val start = subagents.minOf { it.startedAt }
    val running = subagents.any { it.isRunning }
    val end = if (running) now else subagents.mapNotNull { it.endedAt }.maxOrNull() ?: start
    return subagentDuration(end - start)
}

/** Whether a card still has a subagent at work: only then does its header follow the reader. */
internal fun List<Entry>.activeSubagentCard(): Entry? = lastOrNull { entry ->
    entry.kind == "subagents" && entry.subagents.any { it.isRunning }
}

/**
 * The subagents one turn started: a header with their progress and time, and a row for each with
 * its task, what it does now and how long it has taken. The list opens while one works.
 */
@Composable internal fun SubagentCard(entry: Entry) {
    val subagents = entry.subagents
    val running = subagents.count { it.isRunning }
    val failed = subagents.count { it.shownState == "failed" }
    var choice by remember(entry.id) { mutableStateOf<Boolean?>(null) }
    val open = choice ?: (running > 0)
    val now = rememberNow(running > 0)
    if (subagents.isNotEmpty()) Column(Modifier.fillMaxWidth().animateContentSize(Motion.layoutSize),
        verticalArrangement = Arrangement.spacedBy(6.dp)) {
        val (source, pressed) = rememberPress()
        Row(Modifier.pressScale(pressed).clickable(source, null, role = Role.Button,
            onClick = { choice = !open }).padding(vertical = 2.dp), verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(tr("Subagents"), style = Type.chatSubheadline.medium, color = Palette.text)
            SubagentProgress(subagents)
            if (failed > 0) Text("${tr("Failed")} · $failed", style = Type.caption2, color = Palette.danger)
            Text(subagentDuration(subagents, now), style = Type.caption2.monospacedDigit, color = Palette.tertiary)
            Symbol(if (open) "chevron.down" else "chevron.right", size = 12.dp, tint = Palette.tertiary)
        }
        if (open) Column(Modifier.padding(start = 4.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            for (subagent in subagents) SubagentRow(subagent, now)
        }
    }
}

/** One subagent's row: its state, task and newest step, and how long it has taken. */
@Composable private fun SubagentRow(subagent: EntrySubagent, now: Long) {
    val tint = when (subagent.shownState) {
        "failed" -> Palette.danger
        "running" -> Palette.accent
        else -> Palette.secondary
    }
    val symbol = when (subagent.shownState) {
        "done" -> "checkmark.circle.fill"
        "failed" -> "xmark.circle.fill"
        "stopped" -> "minus.circle"
        else -> "circle.dotted"
    }
    val step = subagent.step.ifEmpty {
        when (subagent.shownState) {
            "done" -> tr("Done")
            "failed" -> tr("Failed")
            "stopped" -> tr("Stopped")
            else -> tr("Working…")
        }
    }
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.Top, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Box(Modifier.width(16.dp).padding(top = 2.dp), contentAlignment = Alignment.Center) { Symbol(symbol, size = 15.dp, tint = tint) }
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(1.dp)) {
            Text(subagent.task.ifEmpty { tr("Subagents") }, style = Type.chatFootnote, color = Palette.text,
                maxLines = 1, overflow = TextOverflow.Ellipsis)
            Text(step, style = Type.caption2, color = if (subagent.shownState == "failed") Palette.danger else Palette.secondary,
                maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
        val end = subagent.endedAt ?: if (subagent.isRunning) now else subagent.startedAt
        Text(subagentDuration(end - subagent.startedAt), style = Type.caption2.monospacedDigit, color = Palette.tertiary)
    }
}

/** A ring of the subagents that finished, and "{finished} of {total}". */
@Composable private fun SubagentProgress(subagents: List<EntrySubagent>) {
    val finished = subagents.count { !it.isRunning }
    val label = tr("{finished} of {total}")
        .replace("{finished}", finished.toString())
        .replace("{total}", subagents.count.toString())
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
        Canvas(Modifier.size(14.dp).semantics { contentDescription = label }) {
            val stroke = 2.5.dp.toPx()
            drawArc(Palette.tertiary.copy(alpha = 0.3f), -90f, 360f, false, style = Stroke(stroke))
            drawArc(Palette.accent, -90f, 360f * finished / subagents.size.coerceAtLeast(1), false,
                style = Stroke(stroke, cap = StrokeCap.Round))
        }
        Text(label, style = Type.caption2.monospacedDigit, color = Palette.secondary)
    }
}

/**
 * The header of a subagent card the reader scrolled away from while its subagents work: what it does
 * now, how far it is and how long it has taken. It floats under the chat header and takes no height;
 * a tap scrolls back to the card.
 */
@Composable internal fun SubagentDock(entry: Entry, reveal: () -> Unit) {
    val subagents = entry.subagents
    val running = subagents.filter { it.isRunning }
    val now = rememberNow(running.isNotEmpty())
    val current = running.firstOrNull()
    val (source, pressed) = rememberPress()
    if (running.isNotEmpty()) Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp).pressScale(pressed)
        .clip(RoundedRectangle(14.dp)).background(Palette.surface)
        .clickable(source, null, role = Role.Button, onClick = reveal)
        .semantics { contentDescription = tr("Go to the subagents") }.padding(horizontal = 12.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(tr("Subagents"), style = Type.caption.medium, color = Palette.text)
        SubagentProgress(subagents)
        if (current != null) {
            Symbol("circle.dotted", size = 11.dp, tint = Palette.accent)
            Text(current.step.ifEmpty { tr("Working…") }, Modifier.weight(1f), style = Type.caption2,
                color = Palette.secondary, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
        Text(subagentDuration(subagents, now), style = Type.caption2.monospacedDigit, color = Palette.tertiary)
        Symbol("chevron.right", size = 12.dp, tint = Palette.tertiary)
    }
}

/**
 * The dock over the chat: the newest turn whose subagents still work, while its card is out of view.
 * An overlay, so showing it never moves the messages.
 */
@Composable internal fun SubagentDockHost(entries: List<Entry>, isVisible: (String) -> Boolean, reveal: (String) -> Unit) {
    val card = entries.activeSubagentCard() ?: return
    Box(Modifier.fillMaxSize()) { if (!isVisible(card.id)) SubagentDock(card) { reveal(card.id) } }
}
