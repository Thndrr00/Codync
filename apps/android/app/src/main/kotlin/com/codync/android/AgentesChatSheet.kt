package com.codync.android

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.codync.android.core.ApprovalRequest
import com.codync.android.core.EncryptedChannel
import com.codync.android.core.PresenceStatus
import com.codync.android.core.SetupChecklist
import com.codync.android.core.StretchCard
import com.codync.android.core.approvalCard
import com.codync.android.core.decideApproval
import com.codync.android.core.presence
import com.codync.android.core.setPresence
import com.codync.android.core.setup
import com.codync.android.core.stretchCard
import com.codync.android.core.vaultMask
import com.codync.android.design.CodyncSheet
import com.codync.android.design.Palette
import com.codync.android.design.PlainTextField
import com.codync.android.design.PrimaryButton
import com.codync.android.design.SecondaryButton
import com.codync.android.design.Type
import kotlinx.coroutines.launch

/**
 * The agentes.chat behaviors in one sheet: setup, presence, stretch and approval cards, and the vault.
 * The view holds no I/O: every call goes through the typed channel extensions in core.
 */
@Composable
internal fun AgentesChatSheet(channel: EncryptedChannel, botId: String, isPresented: Boolean, onDismiss: () -> Unit) {
    var setup by remember { mutableStateOf<SetupChecklist?>(null) }
    var presence by remember { mutableStateOf<PresenceStatus?>(null) }
    var status by remember { mutableStateOf("") }
    var card by remember { mutableStateOf<StretchCard?>(null) }
    var approval by remember { mutableStateOf<ApprovalRequest?>(null) }
    var secret by remember { mutableStateOf("") }
    var masked by remember { mutableStateOf("") }
    var error by remember { mutableStateOf("") }
    val scope = rememberCoroutineScope()

    LaunchedEffect(botId, isPresented) {
        if (!isPresented) return@LaunchedEffect
        try {
            setup = channel.setup(botId)
            presence = channel.presence(botId)
        } catch (e: Exception) { error = e.message ?: "error" }
    }

    CodyncSheet(isPresented, onDismiss) {
        Column(verticalArrangement = Arrangement.spacedBy(12.dp), modifier = Modifier.padding(16.dp)) {
            if (error.isNotEmpty()) Text(error, color = Palette.danger)

            Section("Setup") {
                setup?.let { s ->
                    s.steps.forEach { step ->
                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            Text(if (step.done) "✓" else "○")
                            Text(step.label)
                        }
                    }
                    Text("${s.done}/${s.total} done", color = Palette.secondary)
                } ?: Text("Loading…", color = Palette.secondary)
            }

            Section("Presence") {
                Text(presence?.text ?: "No status set", color = Palette.secondary)
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    PlainTextField(text = status, onChange = { status = it }, prompt = "Back at 3")
                    SecondaryButton(title = "Set") {
                        scope.launch {
                            try { presence = channel.setPresence(botId, status); status = "" }
                            catch (e: Exception) { error = e.message ?: "error" }
                        }
                    }
                }
            }

            Section("Cards") {
                SecondaryButton(title = "Preview cards") {
                    scope.launch {
                        try {
                            card = channel.stretchCard("the bot", "runs")
                            approval = channel.approvalCard("the bot", "needs to send an email", "request-1")
                        } catch (e: Exception) { error = e.message ?: "error" }
                    }
                }
                card?.let { Text("${it.type} · ${it.bodyCode}", color = Palette.secondary) }
                approval?.let { a ->
                    Text(a.reason)
                    if (a.status == "pending") {
                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            a.options.forEach { option ->
                                SecondaryButton(title = option.label) {
                                    scope.launch {
                                        try { approval = channel.decideApproval(a, option.id) }
                                        catch (e: Exception) { error = e.message ?: "error" }
                                    }
                                }
                            }
                        }
                    } else { Text("Answered: ${a.selected}", color = Palette.secondary) }
                }
            }

            Section("Vault") {
                PlainTextField(text = secret, onChange = { secret = it }, prompt = "secret")
                PrimaryButton(title = "Mask") {
                    scope.launch {
                        try { masked = channel.vaultMask("openai", secret).masked }
                        catch (e: Exception) { error = e.message ?: "error" }
                    }
                }
                if (masked.isNotEmpty()) Text(masked, color = Palette.secondary)
            }
        }
    }
}

@Composable
private fun Section(title: String, content: @Composable () -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(title, style = Type.titleSmall)
        content()
    }
}
