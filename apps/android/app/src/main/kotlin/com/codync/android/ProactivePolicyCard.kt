package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import com.codync.android.core.BotDraft
import com.codync.android.core.ProactiveSettings
import com.codync.android.core.WORKER_ROLE_TEMPLATES
import com.codync.android.design.*
import com.kyant.shapes.RoundedRectangle

private data class ActionClassSpec(val id: String, val label: String, val detail: String, val defaultPolicy: String)

private val ACTION_CLASS_SPECS = listOf(
    ActionClassSpec("files", "Files", "Read, write or edit workspace files", "ask"),
    ActionClassSpec("shell", "Shell", "Run commands and background jobs", "ask"),
    ActionClassSpec("browse", "Web fetch", "Fetch public URLs and search", "allow"),
    ActionClassSpec("browser", "Browser control", "Drive pages and click controls", "ask"),
    ActionClassSpec("apps", "Connected apps", "Call MCP connectors and secrets", "allow"),
    ActionClassSpec("bots", "Team bots", "Delegate to or message other bots", "ask"),
    ActionClassSpec("routines", "Routines", "Create or modify scheduled routines", "ask"),
)

private val POLICY_CHOICES = listOf(
    "allow" to "Allow",
    "ask" to "Ask",
    "block" to "Block",
)

/**
 * Configures the 12 geometric robot avatar looks (Idea 21), 7 semantic action-class guardrails
 * (Ideas 43 & 44), and proactive routine settings with quiet hours (Idea 41).
 */
@Composable
internal fun ProactivePolicySection(draft: BotDraft, change: (BotDraft) -> Unit) {
    val proactive = draft.proactive ?: ProactiveSettings()

    CardSection("Worker Role Profiles (N15)") {
        row {
            Row(
                modifier = Modifier.horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                for (tpl in WORKER_ROLE_TEMPLATES) {
                    val mappedRules = tpl.actionRules.entries.associate { (k, v) ->
                        k.name.lowercase() to v.name.lowercase()
                    }
                    Row(
                        modifier = Modifier
                            .clip(RoundedRectangle(12.dp))
                            .background(Palette.surface)
                            .clickable(role = Role.Button) {
                                change(
                                    draft.copy(
                                        description = draft.description.ifBlank { tpl.standingInstructions },
                                        avatarLook = tpl.avatarLook,
                                        avatarColor = tpl.avatarColor,
                                        actionRules = draft.actionRules + mappedRules,
                                    )
                                )
                            }
                            .padding(horizontal = 10.dp, vertical = 6.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                    ) {
                        RobotAvatar(look = tpl.avatarLook, color = tpl.avatarColor, size = 20.dp)
                        Text(
                            text = tpl.label,
                            style = Type.caption.semibold,
                            color = Palette.text,
                        )
                    }
                }
            }
        }
    }

    CardSection("Robot Look (12 Geometric Heads)") {
        row {
            Row(
                modifier = Modifier.horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                for (look in ROBOT_AVATAR_LOOKS) {
                    val active = draft.avatarLook == look
                    Row(
                        modifier = Modifier
                            .clip(RoundedRectangle(12.dp))
                            .background(if (active) Palette.accentFill.copy(alpha = 0.18f) else Palette.surface)
                            .clickable(role = Role.Button) {
                                change(draft.copy(avatarLook = if (active) null else look))
                            }
                            .padding(horizontal = 10.dp, vertical = 6.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                    ) {
                        RobotAvatar(look = look, color = draft.avatarColor, size = 24.dp)
                        Text(
                            text = look.replaceFirstChar { it.uppercase() },
                            style = Type.caption.semibold,
                            color = if (active) Palette.accent else Palette.text,
                        )
                    }
                }
            }
        }
    }

    CardSection("Action Class Guardrails") {
        for (spec in ACTION_CLASS_SPECS) {
            row {
                val current = draft.actionRules[spec.id] ?: spec.defaultPolicy
                ValueRow(spec.label, detail = spec.detail) {
                    ChoicePicker(
                        current,
                        POLICY_CHOICES,
                        { next -> change(draft.copy(actionRules = draft.actionRules + (spec.id to next))) },
                        label = spec.label,
                    )
                }
            }
        }
    }

    CardSection("Proactive Heartbeat & Quiet Hours") {
        row {
            SwitchRow(
                "Proactive check-ins",
                "Periodically reviews open items and follow-ups while respecting quiet hours.",
                proactive.enabled,
                { change(draft.copy(proactive = proactive.copy(enabled = it))) },
            )
        }
        if (proactive.enabled) {
            row {
                ValueRow("Check-in interval") {
                    ChoicePicker(
                        proactive.everyHours.toString(),
                        listOf("1" to "Every 1h", "2" to "Every 2h", "4" to "Every 4h", "8" to "Every 8h", "12" to "Every 12h"),
                        { change(draft.copy(proactive = proactive.copy(everyHours = it.toIntOrNull() ?: 4))) },
                        label = "Interval",
                    )
                }
            }
            row {
                ValueRow("Quiet hours", detail = "No proactive messages between ${proactive.quietStart}:00 and ${proactive.quietEnd}:00") {
                    ChoicePicker(
                        "${proactive.quietStart}-${proactive.quietEnd}",
                        listOf(
                            "22-8" to "22:00 – 08:00",
                            "23-7" to "23:00 – 07:00",
                            "21-9" to "21:00 – 09:00",
                        ),
                        { pair ->
                            val parts = pair.split("-")
                            val start = parts.getOrNull(0)?.toIntOrNull() ?: 22
                            val end = parts.getOrNull(1)?.toIntOrNull() ?: 8
                            change(draft.copy(proactive = proactive.copy(quietStart = start, quietEnd = end)))
                        },
                        label = "Quiet hours",
                    )
                }
            }
            row {
                SwitchRow(
                    "Background mini-apps",
                    "Allow mini-apps to refresh data during proactive turns.",
                    proactive.appsInBackground,
                    { change(draft.copy(proactive = proactive.copy(appsInBackground = it))) },
                )
            }
        }
    }
}
