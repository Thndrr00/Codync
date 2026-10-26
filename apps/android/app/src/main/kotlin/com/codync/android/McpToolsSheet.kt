package com.codync.android

import androidx.compose.animation.animateContentSize
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.codync.android.core.InstalledConnector
import com.codync.android.design.*

private val toolModes = listOf("ask" to "Ask first", "allow" to "Allow", "deny" to "Deny")

@Composable internal fun McpToolsSheet(connector: InstalledConnector, state: AppState, store: AppStore) {
    val scope = rememberCoroutineScope()
    val model = remember(store, state.generation, connector.id) { McpToolsModel(connector.id, store, scope, state.generation) }
    var tool by remember(model) { mutableStateOf("") }
    var mode by remember(model) { mutableStateOf("ask") }
    var receiptId by remember(model) { mutableStateOf("") }
    var revoking by remember(model) { mutableStateOf(false) }
    LaunchedEffect(model) { model.watch() }
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader("${connector.name} tools")
        CardForm(Modifier.weight(1f).animateContentSize(Motion.layoutSize)) {
            CardSection("Tool policies", "Applies to every bot using this connection. Observed read-only tools may run automatically; unknown tools require Ask first. Explicit policies override server hints.") {
                row { PlainInput(tool, { tool = it }, "Exact tool name from an approval card") }
                row { SegmentedChoice(mode, toolModes, { mode = it }) }
                row { PrimaryButton("Save policy", enabled = !model.busy && tool.isNotBlank()) { model.setPolicy(tool.trim(), mode) } }
                for (policy in model.policies) row {
                    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text(policy.tool, style = Type.body.semibold, color = Palette.text)
                        Text(toolModes.firstOrNull { it.first == policy.mode }?.second ?: policy.mode, style = Type.caption, color = Palette.secondary)
                        Text(if (policy.explicit) "Owner override" else "Server default", style = Type.caption, color = Palette.secondary)
                        SecondaryButton("Edit policy", enabled = !model.busy) { tool = policy.tool; mode = policy.mode }
                        SecondaryButton("Restore server default", enabled = !model.busy) { model.resetPolicy(policy.tool) }
                    }
                }
                if (!model.loading && model.policies.isEmpty()) row { Text("No overrides. Observed read-only tools use Allow; unknown tools use Ask first.", style = Type.body, color = Palette.secondary) }
            }
            CardSection("Current requests", "Includes waiting approvals and reserved dispatches. Revocation cannot undo an effect already sent to the server; it does not disable future requests.") {
                row { SecondaryButton("Refresh", enabled = !model.loading && !model.busy) { model.refresh() } }
                if (model.loading) row { Waiting("Loading requests…") }
                if (!model.loading && model.requests.isEmpty()) row { Text("No current requests.", style = Type.body, color = Palette.secondary) }
                for (request in model.requests) row {
                    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text(request.tool, style = Type.body.semibold, color = Palette.text)
                        SelectionContainer { Text("Request ID: ${request.id}", style = Type.caption.monospaced, color = Palette.secondary) }
                        Text("${state.bots.firstOrNull { it.id == request.botId }?.name ?: request.botId} · ${request.status} · Expires ${java.text.DateFormat.getTimeInstance().format(java.util.Date(request.expiresAt))}", style = Type.caption, color = Palette.secondary)
                        SecondaryButton("Review exact arguments", enabled = !model.busy) { model.inspect(request) }
                    }
                }
                if (model.next != null) row { SecondaryButton("Load more", enabled = !model.loading && !model.busy) { model.refresh(true) } }
                row { SecondaryButton("Revoke current requests", enabled = !model.busy) { revoking = true } }
            }
            CardSection("Recover saved result", "Retrieval reads a receipt; it never executes a tool again. A missing receipt means the outcome may be unknown. Inspect upstream state before retrying.") {
                row { PlainInput(receiptId, { receiptId = it }, "Saved request ID") }
                row { SecondaryButton("Retrieve result without running", enabled = receiptId.isNotBlank() && !model.inspecting) { model.retrieveResult(receiptId.trim()) } }
                model.receipt?.let { receipt ->
                    row { SelectionContainer { Text("Request ID: ${receipt.id}", style = Type.caption.monospaced, color = Palette.secondary) } }
                    row { Text("Status: ${receipt.status}", style = Type.body, color = Palette.text) }
                    row { SelectionContainer { Text(receipt.formatted ?: "No receipt saved. Inspect upstream state before retrying.", style = Type.callout.monospaced, color = Palette.text) } }
                }
            }
            if (model.inspecting) CardSection { row { Waiting("Loading request details…") } }
            model.detail?.let { detail ->
                val pending = detail.status == "pending" && model.requests.any { it.id == detail.id && it.expiresAt > System.currentTimeMillis() }
                CardSection(detail.tool, "Bot: ${detail.botId} · ${detail.status}") {
                    row { SelectionContainer { Text(detail.formattedArguments, style = Type.callout.monospaced, color = Palette.text) } }
                    if (pending) {
                        row { PrimaryButton("Approve & run", enabled = !model.busy) { model.decide(detail.id, true) } }
                        row { SecondaryButton("Decline", enabled = !model.busy) { model.decide(detail.id, false) } }
                    }
                    row { SecondaryButton("Close request") { model.closeDetail() } }
                }
            }
            model.notice?.let { CardSection { row { Text(it, style = Type.body, color = Palette.secondary) } } }
            model.error?.let { CardSection { row { ErrorRow(it) } } }
        }
    }
    CodyncDialog("Revoke current requests?", revoking, { revoking = false },
        "Revoke waiting and reserved requests for ${connector.name} across all bots. Effects already sent cannot be undone. Future requests keep their configured policies.") {
        listOf(DialogAction("Revoke requests", destructive = true) { model.revokeAll() })
    }
}
