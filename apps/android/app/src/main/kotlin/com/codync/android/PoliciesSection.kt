package com.codync.android

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
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
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.codync.android.core.fill
import com.codync.android.design.*
import kotlinx.coroutines.launch

/**
 * This computer's policies: named CEL rules with an effect and a message, evaluated before any bot's
 * approval rules, with a what-if that never acts. The host compiles and decides; iOS `PoliciesSheet`.
 */
@Composable
internal fun PoliciesSection(state: AppState, store: AppStore) {
    val scope = rememberCoroutineScope()
    var list by remember { mutableStateOf<PolicyList?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    // The rule in the editor; `id` empty adds one.
    var draft by remember { mutableStateOf<PolicyRule?>(null) }
    var tool by remember { mutableStateOf("shell") }
    var command by remember { mutableStateOf("") }
    var initiator by remember { mutableStateOf("person") }
    var answer by remember { mutableStateOf<PolicyAnswer?>(null) }
    var asking by remember { mutableStateOf(false) }

    fun load() {
        scope.launch {
            try {
                list = store.policyList()
                error = null
            } catch (e: Exception) {
                error = e.message
            }
        }
    }

    fun save(rule: PolicyRule) {
        scope.launch {
            try {
                store.savePolicy(rule)
                draft = null
                load()
            } catch (e: Exception) {
                error = e.message
            }
        }
    }

    LaunchedEffect(state.contextId) { load() }

    CardSection(
        "Policies",
        "A deny rule refuses first. An allow rule permits, and once one is saved only what an allow rule names " +
            "goes through; without policies the bots' approval rules and Approvals setting decide as before. " +
            "A rule that cannot run refuses and says why."
    ) {
        val shown = list
        when {
            shown == null -> row { Text("Reading the policies…", style = Type.caption, color = Palette.secondary) }
            shown.rules.isEmpty() -> row {
                Text("No policies: start from a preset, or add a rule of your own.", style = Type.caption, color = Palette.secondary)
            }
            else -> for (rule in shown.rules) {
                row {
                    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                            Text(rule.name, style = Type.body, color = Palette.text, maxLines = 1)
                            Text(rule.expression, style = Type.caption, color = Palette.secondary, maxLines = 2)
                            if (rule.message.isNotBlank()) Text(rule.message, style = Type.caption, color = Palette.tertiary, maxLines = 2)
                        }
                        Text(tr(if (rule.effect == "allow") "Allow" else "Deny"), style = Type.caption.semibold, color = Palette.secondary)
                        CodyncSwitch(rule.enabled, { on ->
                            scope.launch {
                                try {
                                    store.setPolicyEnabled(rule.id, on)
                                    load()
                                } catch (e: Exception) {
                                    error = e.message
                                }
                            }
                        }, Modifier.width(34.dp).semantics { contentDescription = rule.name }) {}
                        IconButton("Edit rule", "pencil") { draft = rule }
                        IconButton("Delete rule", "trash") {
                            scope.launch {
                                try {
                                    store.removePolicy(rule.id)
                                    load()
                                } catch (e: Exception) {
                                    error = e.message
                                }
                            }
                        }
                    }
                }
            }
        }
        row {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                SecondaryButton("New rule") { draft = PolicyRule("", "", "", "deny", "", true) }
                for (preset in shown?.presets ?: emptyList()) {
                    SecondaryButton(fill(tr("Start from “{name}”"), "name" to preset.name)) {
                        draft = PolicyRule("", preset.name, preset.expression, preset.effect, preset.message, true)
                    }
                }
            }
        }
        error?.let { message -> row { Text(message, style = Type.caption, color = Palette.danger) } }
    }

    CardSection(
        "What would happen?",
        "Answered by the same engine a real request goes through, with credentials masked. Nothing runs."
    ) {
        row {
            PlainTextField(tool, { tool = it }, Modifier.fillMaxWidth(), prompt = tr("Tool"), singleLine = true)
        }
        row {
            PlainTextField(command, { command = it; answer = null }, Modifier.fillMaxWidth(), prompt = tr("Command or input"), minLines = 1, maxLines = 3)
        }
        row {
            ValueRow(tr("Who asks")) {
                ChoicePicker(initiator, listOf(
                    "person" to tr("A person"),
                    "routine" to tr("A routine"),
                    "channel" to tr("A channel"),
                    "network" to tr("Another computer"),
                ), { initiator = it }, label = tr("Who asks"))
            }
        }
        row {
            SecondaryButton(if (asking) tr("Asking…") else tr("Ask"), enabled = !asking && tool.isNotBlank()) {
                scope.launch {
                    asking = true
                    try {
                        answer = store.previewPolicy(tool.trim(), command, initiator)
                        error = null
                    } catch (e: Exception) {
                        error = e.message
                    } finally {
                        asking = false
                    }
                }
            }
        }
        answer?.let { result ->
            row {
                Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text(decisionLine(result), style = Type.body.semibold, color = Palette.text)
                    Text(result.explanation, style = Type.caption, color = Palette.secondary)
                    for ((name, value) in result.variables) {
                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                            Text(name, style = Type.caption, color = Palette.tertiary, modifier = Modifier.weight(1f, fill = false))
                            Text(value.ifBlank { "—" }, style = Type.caption, color = Palette.secondary, maxLines = 1)
                        }
                    }
                }
            }
        }
    }

    CodyncSheet(isPresented = draft != null, onDismiss = { draft = null }) {
        draft?.let { rule -> PolicyEditor(rule, onSave = { save(it) }, onCancel = { draft = null }) }
    }
}

@Composable
private fun decisionLine(answer: PolicyAnswer): String = when (answer.decision) {
    "allow" -> tr("Allowed")
    "deny" -> answer.ruleName?.let { fill(tr("Refused by “{name}”"), "name" to it) } ?: tr("Refused")
    else -> tr("No policy decides: the bot's approval rules do")
}

/** One rule: its name, expression, effect and message. Saving asks the host, which compiles it. */
@Composable
private fun PolicyEditor(rule: PolicyRule, onSave: (PolicyRule) -> Unit, onCancel: () -> Unit) {
    var name by remember(rule.id, rule.name) { mutableStateOf(rule.name) }
    var expression by remember(rule.id, rule.expression) { mutableStateOf(rule.expression) }
    var effect by remember(rule.id, rule.effect) { mutableStateOf(rule.effect) }
    var message by remember(rule.id, rule.message) { mutableStateOf(rule.message) }
    val ready = expression.isNotBlank()
    Column(Modifier.fillMaxWidth().padding(20.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
        ModalHeader(if (rule.id.isEmpty()) tr("New policy") else tr("Edit policy")) {
            IconButton("Close", "xmark", action = onCancel)
        }
        Text(
            tr("The expression is CEL and must be true or false. It reads tool.name, tool.kind, intent, command, " +
                "file.path, bot.name, actor.role, initiator.kind, key and more."),
            style = Type.caption,
            color = Palette.secondary,
        )
        PlainTextField(name, { name = it }, Modifier.fillMaxWidth(), prompt = tr("Name"), singleLine = true)
        PlainTextField(expression, { expression = it }, Modifier.fillMaxWidth(), prompt = tr("Expression"),
            style = Type.body, minLines = 2, maxLines = 6)
        ChoicePicker(effect, listOf("deny" to tr("Deny"), "allow" to tr("Allow")), { effect = it }, label = tr("Effect"))
        PlainTextField(message, { message = it }, Modifier.fillMaxWidth(), prompt = tr("What a person reads when this rule decides"),
            minLines = 1, maxLines = 3)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            PrimaryButton(tr("Save"), enabled = ready) {
                onSave(rule.copy(name = name, expression = expression, effect = effect, message = message))
            }
            SecondaryButton(tr("Cancel"), action = onCancel)
        }
    }
}
