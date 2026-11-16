package com.codync.android

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.codync.android.core.LinkState
import com.codync.android.core.HostHealth
import com.codync.android.core.SkillUsage
import com.codync.android.core.SkillUsageRow
import com.codync.android.core.UsageProvider
import com.codync.android.core.UsageReport
import com.codync.android.core.WeekSummary
import com.codync.android.core.UsageWindow
import com.codync.android.design.*
import com.kyant.shapes.RoundedRectangle
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle
import kotlin.math.roundToInt

/**
 * Usage limits, opened from the State tab (iOS `UsageSheet` + `UsageView`): one card per
 * provider, the tightest limit as a big bar, every limit listed below. Pull down to refresh.
 */
@Composable fun UsageScreen(state: AppState, refresh: (force: Boolean) -> Unit, reload: suspend () -> Unit,
    week: suspend () -> WeekSummary? = { null }, store: AppStore? = null) {
    val dismiss = LocalDismissModal.current
    val report = remember(state.usage) { runCatching { UsageReport.decode(state.usage) }.getOrNull() ?: UsageReport() }
    val online = state.actualConnection is LinkState.Ready
    // Opening Usage asks the computer for a current report (it answers from cache when it can).
    LaunchedEffect(online) { if (online) refresh(false) }
    var summary by remember { mutableStateOf<WeekSummary?>(null) }
    LaunchedEffect(online) { if (online) summary = week() }
    var health by remember { mutableStateOf<HostHealth?>(null) }
    var skills by remember { mutableStateOf<SkillUsage?>(null) }
    LaunchedEffect(online) {
        if (!online || store == null) return@LaunchedEffect
        health = store.hostHealth() ?: health
        skills = store.skillUsage() ?: skills
    }
    var tab by rememberSaveable { mutableStateOf(UsageTab.Limits) }
    Column(Modifier.fillMaxSize()) {
        ScreenHeader(leading = { IconButton("Close", "xmark", action = dismiss) },
            title = { Text("Usage", style = Type.body.semibold, color = Palette.text) })
        if (state.computers.isEmpty()) {
            EmptyUsage("No computer yet", "Usage shows up once a computer is connected.")
            return@Column
        }
        PullToRefresh({
            if (online) {
                reload()
                store?.let {
                    health = it.hostHealth() ?: health
                    skills = it.skillUsage() ?: skills
                }
            }
        }, Modifier.weight(1f)) {
            Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
                health?.let { HealthCard(it) }
                Text(tr("Breakdown"), Modifier.fillMaxWidth(), style = Type.caption, color = Palette.tertiary)
                TabChoice(tab, listOf(UsageTab.Limits to "Limits", UsageTab.Skills to "Skills"), { tab = it })
                when (tab) {
                    UsageTab.Limits -> {
                        for (provider in report.providers) key(provider.id) { ProviderCard(provider) }
                        summary?.takeIf { it.activeBots > 0 }?.let { WeekCard(it) }
                        if (report.providers.isEmpty()) EmptyUsage("No usage yet", "Your computer reads plan limits from the agents installed on it.")
                    }
                    UsageTab.Skills -> SkillCard(skills)
                }
            }
        }
    }
}

private enum class UsageTab { Limits, Skills }

/** The computer's RAM, CPU and disk, read from `hostHealth`. */
@Composable private fun HealthCard(health: HostHealth) {
    Column(Modifier.fillMaxWidth().clip(RoundedRectangle(20.dp)).background(Palette.surface).padding(14.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp)) {
        for ((label, value) in health.lines()) DetailRow(tr(label), value)
    }
}

/** How often each skill's text entered a turn, with its estimated tokens, cost and share. */
@Composable private fun SkillCard(usage: SkillUsage?) {
    Column(Modifier.fillMaxWidth().clip(RoundedRectangle(20.dp)).background(Palette.surface).padding(14.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp)) {
        val rows = usage?.skills.orEmpty()
        if (rows.isEmpty()) {
            Text(tr("No skill has been loaded yet."), style = Type.subheadline, color = Palette.secondary)
        } else {
            for (row in rows) SkillRow(row)
        }
        Text(tr("A load is counted when an agent reads a skill's SKILL.md during a turn."), style = Type.caption2, color = Palette.tertiary)
    }
}

@Composable private fun SkillRow(row: SkillUsageRow) {
    Column(verticalArrangement = Arrangement.spacedBy(3.dp)) {
        Row {
            Text(row.name, Modifier.weight(1f), style = Type.subheadline.semibold, color = Palette.text, maxLines = 1)
            Text(costText(row.costUsd), style = Type.footnote.monospacedDigit,
                color = if (row.costUsd == null) Palette.secondary else Palette.text)
        }
        val loads = "${row.loads} ${tr("loads")}"
        val tokens = "${row.tokens} ${tr("tokens")}"
        val share = "${Math.round(row.share * 100)}%"
        val agents = "${row.agents} ${tr("agents")}"
        Text(listOf(loads, tokens, share, agents).joinToString(" · "), style = Type.caption, color = Palette.secondary)
    }
}

private fun costText(cost: Double?): String = when {
    cost == null -> tr("Unavailable")
    cost == 0.0 -> "$0.00"
    cost < 0.01 -> tr("Under $0.01")
    else -> String.format(Locale.US, "$%.2f", cost)
}

/** The last seven days, as counts: what was said, done and approved. */
@Composable private fun WeekCard(week: WeekSummary) {
    Column(Modifier.fillMaxWidth().clip(RoundedRectangle(20.dp)).background(Palette.surface).padding(14.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(tr("Your week"), style = Type.subheadline.semibold, color = Palette.text)
        DetailRow("Messages you sent", week.messages.toString())
        DetailRow("Replies from your bots", week.replies.toString())
        DetailRow("Tool calls", week.tools.toString())
        DetailRow("File edits", week.fileEdits.toString())
        DetailRow("Approvals answered", "${week.approvalsAnswered} / ${week.approvalsAsked}")
        if (week.approvalsWaiting > 0) DetailRow("Approvals still waiting", week.approvalsWaiting.toString())
        val anonymous = tr("Bot")
        if (week.topBots.isNotEmpty()) Text("${tr("Busiest")}: ${week.topBots.joinToString { it.name.ifBlank { anonymous } }}",
            style = Type.caption, color = Palette.tertiary)
    }
}

@Composable private fun EmptyUsage(title: String, message: String) {
    Column(Modifier.fillMaxWidth().padding(top = 60.dp, start = 24.dp, end = 24.dp), horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Symbol("chart.bar", size = 48.dp, tint = Palette.tertiary)
        Text(tr(title), style = Type.title3.semibold, color = Palette.text, textAlign = TextAlign.Center)
        Text(tr(message), style = Type.subheadline, color = Palette.secondary, textAlign = TextAlign.Center)
    }
}

@Composable private fun ProviderCard(provider: UsageProvider) {
    var collapsed by rememberSaveable(provider.id) { mutableStateOf(false) }
    val turn by animateFloatAsState(if (collapsed) 180f else 0f, Motion.layout(), label = "Collapse")
    val tint = provider.widgetTint()
    Column(Modifier.fillMaxWidth().clip(RoundedRectangle(20.dp)).background(Palette.surface).padding(14.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            Box(Modifier.size(34.dp).clip(RoundedRectangle(10.dp)).background(provider.tint().copy(alpha = 0.12f)), contentAlignment = Alignment.Center) {
                ProviderMascot(provider, 22.dp)
            }
            Text(provider.name, Modifier.weight(1f), style = Type.subheadline.semibold, color = Palette.text)
            IconButton(if (collapsed) "Expand" else "Collapse", "chevron.up", Modifier.rotate(turn)) { collapsed = !collapsed }
        }
        provider.statusLabel?.let { Text(tr(it), style = Type.caption, color = Palette.secondary) }
        provider.tightest?.let { top ->
            Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Row {
                    Text(tr(top.title), Modifier.weight(1f), style = Type.subheadline, color = Palette.text)
                    Text(percent(top.percent), style = Type.subheadline.monospacedDigit, color = Palette.text)
                }
                UsageTicks(top.percent ?: 0.0, tint, 15.dp)
                val reset = top.resetsShort()
                if (reset != null) Text(resetLine(reset), Modifier.fillMaxWidth(), style = Type.caption.monospacedDigit, color = Palette.tertiary, textAlign = TextAlign.End)
            }
        }
        AnimatedVisibility(!collapsed, enter = fadeIn(Motion.fade()) + expandVertically(Motion.layoutSize),
            exit = fadeOut(Motion.fade()) + shrinkVertically(Motion.layoutSize)) {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                for (window in provider.windows) {
                    DetailRow(tr(window.title), reading(window))
                    val reset = window.resetsShort()
                    if (reset != null) DetailRow(tr("Resets"), resetLine(reset))
                }
                DetailRow("Source", provider.sourceLabel)
                DetailRow("Updated", provider.updatedAt?.takeIf { it > 0 }?.let(::updatedAt) ?: "Not reported")
            }
        }
    }
}

private fun reading(window: UsageWindow) = window.text?.takeIf { it.isNotBlank() } ?: percent(window.percent)

private fun percent(value: Double?) = value?.takeIf { it.isFinite() }?.let { "${it.roundToInt()}%" } ?: "—"

/** "Oct 17, 2026 at 10:20 PM", as iOS formats an abbreviated date with a short time. */
private fun updatedAt(milliseconds: Long): String {
    val at = Instant.ofEpochMilli(milliseconds).atZone(ZoneId.systemDefault())
    val date = at.format(DateTimeFormatter.ofLocalizedDate(FormatStyle.MEDIUM))
    val time = at.format(DateTimeFormatter.ofLocalizedTime(FormatStyle.SHORT))
    return if (java.util.Locale.getDefault().language == "en") "$date at $time" else "$date $time"
}

@Composable private fun DetailRow(label: String, value: String) {
    Row {
        Text(tr(label), Modifier.weight(1f), style = Type.footnote, color = Palette.secondary)
        Text(value, style = Type.footnote.monospacedDigit, color = Palette.text)
    }
}
