package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.codync.android.core.SentinelIssue
import com.codync.android.core.SentinelReport
import com.codync.android.design.*
import kotlinx.coroutines.CancellationException
import java.text.DateFormat
import java.util.Date

/** The host's runtime scan, distinct from the bot setup check below it. */
@Composable internal fun SentinelDiagnosticsSheet(state: AppState, store: AppStore) {
    var loading by remember { mutableStateOf(true) }
    var report by remember { mutableStateOf<SentinelReport?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var attempt by remember { mutableStateOf(0) }
    LaunchedEffect(state.contextId, store.channel, attempt) {
        loading = true
        report = null
        error = null
        try {
            report = store.sentinelReport()
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            error = e.message ?: "Could not run Sentinel diagnostic."
        } finally {
            loading = false
        }
    }
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader("System diagnostics")
        CardForm(padding = PaddingValues(20.dp)) {
            CardSection("Sentinel Watchdog", "Reviews recent tool errors, failed turns, retry loops, and queue waits.") {
                row {
                    ValueRow("Status", when {
                        loading -> "Scanning…"
                        error != null -> "Error"
                        report?.healthy == true -> "Healthy"
                        report != null -> "${report!!.issues.size} issue(s)"
                        else -> "Not scanned"
                    })
                }
                report?.let { scan ->
                    row { ValueRow("Scanned", sentinelTime(scan.scannedAt)) }
                    row { ValueRow("Entries examined", scan.entriesExamined.toString()) }
                    row { ValueRow("Lookback", "${scan.lookbackMs / 3_600_000} hours") }
                }
                error?.let { row { Text(it, style = Type.footnote, color = Palette.danger) } }
                row { ActionRow("arrow.clockwise", if (error != null) "Retry scan" else "Scan again", !loading) { attempt += 1 } }
            }
            report?.let { scan ->
                CardSection("Diagnostic findings") {
                    if (scan.healthy) {
                        row { Text("No runtime issues found in the scanned entries.", style = Type.subheadline, color = Palette.secondary) }
                    } else {
                        for (issue in scan.issues) row { SentinelFinding(issue, state) }
                    }
                }
            }
            BotMaintenance(state, store)
        }
    }
}

@Composable private fun SentinelFinding(issue: SentinelIssue, state: AppState) {
    val botName = state.bots.firstOrNull { it.id == issue.botId }?.name ?: issue.botId
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text(issue.title, style = Type.subheadline.semibold, color = when (issue.severity) {
            "high" -> Palette.danger
            "medium" -> Palette.warning
            else -> Palette.text
        })
        Text("${issue.kind.replace('_', ' ')} · ${issue.severity}", style = Type.footnote, color = Palette.secondary)
        Text(listOfNotNull(botName, issue.toolName).joinToString(" · "), style = Type.footnote, color = Palette.secondary)
        Text("${issue.count} occurrences · ${issue.wastedTurns} wasted turns", style = Type.footnote, color = Palette.secondary)
        Text("First seen: ${sentinelTime(issue.firstSeenAt)}", style = Type.footnote, color = Palette.secondary)
        Text("Last seen: ${sentinelTime(issue.lastSeenAt)}", style = Type.footnote, color = Palette.secondary)
        Text("Evidence entry: ${issue.evidenceEntryId}", style = Type.footnote, color = Palette.secondary)
    }
}

private fun sentinelTime(milliseconds: Long): String =
    if (milliseconds > 0) DateFormat.getDateTimeInstance(DateFormat.SHORT, DateFormat.SHORT).format(Date(milliseconds)) else "Unknown"
