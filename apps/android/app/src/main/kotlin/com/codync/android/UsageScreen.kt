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
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.codync.android.core.LinkState
import com.codync.android.core.UsageProvider
import com.codync.android.core.UsageReport
import com.codync.android.core.WeekSummary
import com.codync.android.core.UsageWindow
import com.codync.android.design.*
import com.kyant.shapes.RoundedRectangle
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.intOrNull
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.longOrNull
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle
import kotlin.math.roundToInt

/** An additive part of the host's usage event: a spent window that can spend one extra credit. */
private data class CreditOffer(val id: String, val credits: Int)
private data class UsingCredits(val id: String, val until: Long)

private fun JsonObject.creditOffers(): List<CreditOffer> = (this["creditOffers"]?.jsonArray).orEmpty().mapNotNull { item ->
    val row = item.jsonObject
    val id = row["id"]?.jsonPrimitive?.content ?: return@mapNotNull null
    CreditOffer(id, row["credits"]?.jsonPrimitive?.intOrNull ?: 0)
}

private fun JsonObject.usingCredits(): List<UsingCredits> = (this["usingCredits"]?.jsonArray).orEmpty().mapNotNull { item ->
    val row = item.jsonObject
    val id = row["id"]?.jsonPrimitive?.content ?: return@mapNotNull null
    val until = row["until"]?.jsonPrimitive?.longOrNull ?: return@mapNotNull null
    UsingCredits(id, until)
}

/**
 * Usage limits, opened from the State tab (iOS `UsageSheet` + `UsageView`): one card per
 * provider, the tightest limit as a big bar, every limit listed below. Pull down to refresh.
 * `reset` calls the host's Reset usage method; the host holds every credential and credit count.
 */
@Composable fun UsageScreen(state: AppState, refresh: (force: Boolean) -> Unit, reload: suspend () -> Unit,
    week: suspend () -> WeekSummary? = { null }, reset: suspend (String) -> Unit = {}) {
    val dismiss = LocalDismissModal.current
    val report = remember(state.usage) { runCatching { UsageReport.decode(state.usage) }.getOrNull() ?: UsageReport() }
    val online = state.actualConnection is LinkState.Ready
    // Opening Usage asks the computer for a current report (it answers from cache when it can).
    LaunchedEffect(online) { if (online) refresh(false) }
    var summary by remember { mutableStateOf<WeekSummary?>(null) }
    LaunchedEffect(online) { if (online) summary = week() }
    val offers = remember(state.usage) { state.usage.creditOffers() }
    val usingCredits = remember(state.usage) { state.usage.usingCredits() }
    Column(Modifier.fillMaxSize()) {
        ScreenHeader(leading = { IconButton("Close", "xmark", action = dismiss) },
            title = { Text("Usage", style = Type.body.semibold, color = Palette.text) })
        if (state.computers.isEmpty()) {
            EmptyUsage("No computer yet", "Usage shows up once a computer is connected.")
            return@Column
        }
        PullToRefresh({ if (online) reload() }, Modifier.weight(1f)) {
            Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
                for (provider in report.providers) key(provider.id) {
                    val baseId = provider.id.substringBefore(':')
                    ProviderCard(provider, offers.firstOrNull { it.id.substringBefore(':') == baseId },
                        usingCredits.firstOrNull { it.id.substringBefore(':') == baseId }, reset, reload)
                }
                summary?.takeIf { it.activeBots > 0 }?.let { WeekCard(it) }
                if (report.providers.isEmpty()) EmptyUsage("No usage yet", "Your computer reads plan limits from the agents installed on it.")
            }
        }
    }
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

@Composable private fun ProviderCard(provider: UsageProvider, offer: CreditOffer?, using: UsingCredits?,
    reset: suspend (String) -> Unit, reload: suspend () -> Unit) {
    var collapsed by rememberSaveable(provider.id) { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
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
        if (using != null) Text("${tr("Using credits")} · ${updatedAt(using.until)}",
            style = Type.caption, color = Palette.secondary)
        if (offer != null) SecondaryButton(tr("Reset usage")) {
            scope.launch { reset(offer.id); reload() }
        }
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
