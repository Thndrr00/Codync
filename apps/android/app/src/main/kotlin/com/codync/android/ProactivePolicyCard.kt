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

    CardSection("Action rules and proactive check-ins") {
        row {
            Text(tr("Action-class rules and proactive check-ins are not supported by this host. These settings are unavailable."),
                style = Type.caption, color = Palette.secondary)
        }
    }
}
