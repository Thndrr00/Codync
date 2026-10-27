package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
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

/** Role instructions and avatar choices supported by the host. */
@Composable
internal fun ProactivePolicySection(draft: BotDraft, change: (BotDraft) -> Unit) {
    CardSection("Worker role profiles") {
        row {
            Row(
                modifier = Modifier.horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                for (tpl in WORKER_ROLE_TEMPLATES) {
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

    ProactiveCheckIns(draft, change)
}

private val INTERVALS = listOf(1, 2, 4, 8, 12, 24)
private val DAILY_LIMITS = listOf(1, 2, 3, 5, 8)

private fun hourOptions() = (0..23).map { it.toString() to "%02d:00".format(it) }

/** How often the bot may wake on its own, when it stays quiet and how many times a day (the host runs them). */
@Composable
private fun ProactiveCheckIns(draft: BotDraft, change: (BotDraft) -> Unit) {
    val settings = draft.proactive ?: ProactiveSettings()
    fun update(next: ProactiveSettings) = change(draft.copy(proactive = next))
    CardSection("Proactive check-ins", footer = "The bot wakes up on its own to review what is open and only writes to you when it has something to say.") {
        row { SwitchRow("Proactive check-ins", "Let this bot check in without being asked", settings.enabled, { update(settings.copy(enabled = it)) }) }
        if (settings.enabled) {
            row {
                ValueRow("Check in every") {
                    ChoicePicker(settings.everyHours.toString(), INTERVALS.map { it.toString() to if (it == 1) "1 hour" else "$it hours" },
                        { update(settings.copy(everyHours = it.toInt())) }, label = "Check-in interval")
                }
            }
            row {
                ValueRow("Quiet from") {
                    ChoicePicker(settings.quietStart.toString(), hourOptions(), { update(settings.copy(quietStart = it.toInt())) }, label = "Quiet hours start")
                }
            }
            row {
                ValueRow("Quiet until") {
                    ChoicePicker(settings.quietEnd.toString(), hourOptions(), { update(settings.copy(quietEnd = it.toInt())) }, label = "Quiet hours end")
                }
            }
            row {
                ValueRow("At most per day") {
                    ChoicePicker(settings.maxPerDay.toString(), DAILY_LIMITS.map { it.toString() to it.toString() },
                        { update(settings.copy(maxPerDay = it.toInt())) }, label = "Daily check-in limit")
                }
            }
        }
    }
}
