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
            insights.provider?.let { provider ->
                CardSection("Model") { row { ValueRow("Agent", provider) } }
            }
            CardSection("Today") {
                row {
                    ValueRow("Tokens", if (insights.usage.limit != null) "${insights.usage.usedText} / ${insights.usage.limit}" else insights.usage.usedText)
                }
                row { ValueRow("Skills", insights.skills.total.toString()) }
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
