package com.codync.android

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import com.codync.android.core.Bot
import com.codync.android.design.*
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch

/** What the maintenance sections show, and the one call in flight. */
private class MaintenanceState {
    var report by mutableStateOf<DoctorReport?>(null)
    var budgets by mutableStateOf<List<TokenBudget>>(emptyList())
    var busy by mutableStateOf(false)
    var error by mutableStateOf<String?>(null)
    var notice by mutableStateOf<String?>(null)
}

/**
 * Bot upkeep on the diagnostics sheet, as the desktop's diagnostics page has it: a check of the bots' setup,
 * daily token limits, failed routine runs, and bots as portable bundles.
 */
@Composable internal fun ColumnScope.BotMaintenance(state: AppState, store: AppStore) {
    val bots = state.bots.filter { it.kind != "group" }
    val scope = rememberCoroutineScope()
    val context = LocalContext.current
    val ui = remember { MaintenanceState() }
    var confirmClear by remember { mutableStateOf(false) }
    var budgetScope by remember { mutableStateOf(FLEET_BUDGET) }
    var limit by remember { mutableStateOf("") }
    var shared by remember { mutableStateOf<String?>(null) }
    var bundle by remember { mutableStateOf("") }
    var folder by remember { mutableStateOf("") }
    val online = state.actualConnection is com.codync.android.core.LinkState.Ready
    val sharedBot = bots.firstOrNull { it.id == shared } ?: bots.firstOrNull()

    fun run(work: suspend () -> Unit) {
        if (ui.busy) return
        ui.busy = true; ui.error = null; ui.notice = null
        scope.launch {
            try { work() } catch (e: CancellationException) { throw e } catch (e: Exception) { ui.error = e.message ?: "That didn't work." }
            finally { ui.busy = false }
        }
    }
    LaunchedEffect(online) { if (online) run { ui.budgets = store.tokenBudgets() } }

    CardSection("Bot check", "Finds bots that can't run as set up: a missing folder, no agent, a group member that was deleted.") {
        row { ActionRow("checkmark.shield", "Check bots", online && !ui.busy) { run { ui.report = store.doctor() } } }
        row { ActionRow("trash", "Clear failed routine runs", online && !ui.busy) { confirmClear = true } }
        ui.report?.let { report ->
            row {
                Text(if (report.errors > 0) "${report.errors} of ${report.checked} bots can't run as set up."
                    else "${report.checked} bots checked. Nothing to fix.", style = Type.subheadline,
                    color = if (report.errors > 0) Palette.warning else Palette.secondary)
            }
            for (finding in report.findings) row {
                Text(finding.message, style = Type.footnote, color = if (finding.isError) Palette.danger else Palette.warning)
            }
        }
    }

    CardSection("Daily token limits", "Counts the tokens each turn used today (not cached reads). At 80% the bot's chat says so; at the limit its routines pause until you turn them back on. Your own messages are never refused.") {
        for (budget in ui.budgets) row {
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                Column(Modifier.weight(1f)) {
                    Text(if (budget.botId == FLEET_BUDGET) "All bots" else bots.firstOrNull { it.id == budget.botId }?.name ?: "A deleted bot", style = Type.body, color = Palette.text)
                    Text("%,d of %,d today".format(budget.used, budget.limit) + when (budget.level) {
                        "exceeded" -> " · limit reached"
                        "warning" -> " · 80% used"
                        else -> ""
                    }, style = Type.footnote, color = if (budget.level == "within") Palette.secondary else Palette.warning)
                }
                Text("Remove", Modifier.plainButton(enabled = online && !ui.busy) { run { ui.budgets = store.setTokenBudget(budget.botId, null) } },
                    style = Type.subheadline, color = Palette.accent)
            }
        }
        if (ui.budgets.isEmpty()) row { Text("No limits set.", style = Type.subheadline, color = Palette.secondary) }
        row {
            ValueRow("Set a limit for") {
                ChoicePicker(budgetScope, listOf(FLEET_BUDGET to "All bots") + bots.map { it.id to it.name }, { budgetScope = it }, label = "Set a limit for")
            }
        }
        row { PlainTextField(limit, { limit = it }, Modifier.fillMaxWidth(), prompt = "Tokens per day, for example 5000000", singleLine = true) }
        val tokens = limit.filter(Char::isDigit).toLongOrNull()
        row { ActionRow("checkmark", "Save limit", online && !ui.busy && tokens != null && tokens > 0) { run { ui.budgets = store.setTokenBudget(budgetScope, tokens) } } }
    }

    CardSection("Share a bot", "A bundle holds the bot's name, instructions, agent, model and fallbacks. It never holds folders, accounts, connectors, history or keys.") {
        row {
            ValueRow("Bot") {
                ChoicePicker(sharedBot?.id.orEmpty(), bots.map { it.id to it.name }, { shared = it }, label = "Bot", enabled = bots.isNotEmpty())
            }
        }
        row {
            ActionRow("doc.on.doc", "Copy bundle", online && !ui.busy && sharedBot != null) {
                run {
                    val text = store.exportBot(sharedBot!!.id)
                    (context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager).setPrimaryClip(ClipData.newPlainText("Codync bot", text))
                    ui.notice = "Bundle copied."
                }
            }
        }
        row { PlainTextField(bundle, { bundle = it }, Modifier.fillMaxWidth(), prompt = "Paste a bundle here", minLines = 3, maxLines = 8) }
        row { PlainTextField(folder, { folder = it }, Modifier.fillMaxWidth(), prompt = "Folder for the imported bots", singleLine = true) }
        row {
            ActionRow("square.and.arrow.down", "Import bots", online && !ui.busy && bundle.isNotBlank() && folder.isNotBlank()) {
                run {
                    val made = store.importBots(bundle, folder.trim())
                    ui.notice = "Imported $made ${if (made == 1) "bot" else "bots"}."
                }
            }
        }
    }

    TeamRunSection(bots, online) { botId, text -> store.sendToBot(botId, text) }

    if (ui.error != null || ui.notice != null) CardSection {
        row { Text(ui.error ?: ui.notice.orEmpty(), style = Type.footnote, color = if (ui.error != null) Palette.danger else Palette.secondary) }
    }
    CodyncDialog("Clear failed routine runs?", confirmClear, { confirmClear = false },
        "Failed runs you can no longer retry are removed from every bot's routine history. Runs waiting to retry stay.") {
        listOf(DialogAction("Clear", destructive = true) {
            confirmClear = false
            run {
                val cleared = store.clearFailedRuns()
                ui.notice = if (cleared > 0) "Cleared $cleared failed ${if (cleared == 1) "run" else "runs"}." else "No failed runs to clear."
            }
        })
    }
}

/** A tappable row with an icon, like the rows of the settings sheet. */
@Composable internal fun ActionRow(icon: String, title: String, enabled: Boolean, action: () -> Unit) {
    Row(Modifier.fillMaxWidth().plainButton(enabled = enabled, action = action), verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        Symbol(icon, size = 20.dp, tint = if (enabled) Palette.text else Palette.tertiary)
        Text(title, style = Type.body, color = if (enabled) Palette.text else Palette.tertiary)
    }
}
