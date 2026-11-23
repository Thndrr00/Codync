package com.codync.android

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.unit.dp
import com.codync.android.core.AppState
import com.codync.android.core.BotInsights
import com.codync.android.design.*

/** A bot's routines, model and daily use, read from the host's `botInsights` method. */
@Composable internal fun BotInsightsView(botId: String, state: AppState, store: AppStore) {
    var data by remember(botId) { mutableStateOf<BotInsights?>(null) }
    var error by remember(botId) { mutableStateOf<String?>(null) }
    LaunchedEffect(botId, state.generation) {
        data = null
        error = null
        try { data = store.botInsights(botId) } catch (e: Exception) { error = e.message }
    }
    CardForm {
        if (error != null) CardSection("Insights") {
            row { Text("Could not load insights.", style = Type.body, color = Palette.secondary) }
        } else if (data == null) CardSection("Insights") {
            row { Text("Loading…", style = Type.body, color = Palette.secondary) }
        } else {
            val insights = data!!
            if (insights.description.isNotBlank()) CardSection("About") {
                row { Text(insights.description, style = Type.caption, color = Palette.secondary) }
            }
            insights.provider?.let { provider ->
                CardSection("Model") { row { ValueRow("Agent", provider) } }
            }
            CardSection("Today") {
                row {
                    ValueRow("Tokens", if (insights.usage.limit != null) "${insights.usage.usedText} / ${insights.usage.limit}" else insights.usage.usedText)
                }
                row { ValueRow("Skills", insights.skills.total.toString()) }
            }
            val activity = insights.activity
            CardSection("Activity") {
                row { Text(activity.headline, style = Type.caption, color = Palette.secondary) }
                row { ValueRow("Steps", activity.steps.toString()) }
                if (activity.errors > 0) row {
                    Text("$activity.errors failed", style = Type.caption, color = Palette.danger)
                }
                if (activity.groups.isNotEmpty()) row {
                    Text(activity.groups.take(6).joinToString(" · ") { "${it.count} ${it.label}" },
                        style = Type.caption, color = Palette.secondary)
                }
                for (day in activity.days.takeLast(5)) row {
                    Text("${day.label}  ${"▮".repeat(day.width.coerceAtLeast(1))}  ${day.count}",
                        style = Type.caption, color = Palette.tertiary)
                }
                for (step in activity.items.take(6)) row {
                    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                        Text(step.text, style = Type.caption,
                            color = if (step.status == "error") Palette.danger else Palette.text)
                        if (step.detail.isNotBlank()) Text(step.detail, style = Type.caption, color = Palette.tertiary)
                    }
                }
            }
            if (insights.recentWork.isNotEmpty()) CardSection("Recent work") {
                for (line in insights.recentWork) row {
                    Text(line, style = Type.caption, color = Palette.secondary)
                }
            }
            CardSection("Routines") {
                if (insights.routines.isEmpty()) row {
                    Text("No routines yet.", style = Type.body, color = Palette.secondary)
                }
                for (routine in insights.routines) {
                    row {
                        Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                            Text(routine.name, style = Type.subheadline.semibold, color = Palette.text)
                            if (!routine.enabled) Text("Paused", style = Type.caption, color = Palette.secondary)
                            if (routine.schedules.isNotEmpty()) Text(routine.schedules.joinToString(" · "), style = Type.caption, color = Palette.secondary)
                            routine.nextIn?.let { Text("Next in $it", style = Type.caption, color = Palette.tertiary) }
                        }
                    }
                }
            }
        }
    }
}
