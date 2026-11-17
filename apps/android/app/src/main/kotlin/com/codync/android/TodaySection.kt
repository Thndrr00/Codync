package com.codync.android

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
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
import androidx.compose.ui.unit.dp
import com.codync.android.design.*
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch

/** The host's brief for today, above the board (iOS `TodaySectionView`). */
@Composable
internal fun TodaySection(store: AppStore, onError: (String?) -> Unit) {
    val scope = rememberCoroutineScope()
    var digest by remember { mutableStateOf<DailyDigest?>(null) }
    var weather by remember { mutableStateOf<WeatherReport?>(null) }
    var place by remember { mutableStateOf("") }
    var editingPlace by remember { mutableStateOf(false) }
    var weatherError by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }

    suspend fun load() {
        busy = true
        try {
            digest = store.dailyDigest()
            onError(null)
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (failure: Exception) {
            onError(failure.message)
        }
        try {
            weather = store.weather()
            weatherError = null
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (failure: Exception) {
            weather = null
            weatherError = failure.message
        }
        busy = false
    }

    fun savePlace() {
        val wanted = place.trim()
        if (wanted.isEmpty()) return
        scope.launch {
            try {
                weather = store.setWeatherPlace(wanted)
                place = ""
                editingPlace = false
                weatherError = null
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (failure: Exception) {
                weatherError = failure.message
            }
        }
    }

    LaunchedEffect(Unit) { load() }

    CardSection("Today", "Read from this computer's board, approvals, calendars and waiting items. No model is asked.") {
        digest?.let { today ->
            row {
                Text(today.text, style = Type.body, color = Palette.text)
            }
            val parts = buildList {
                if (today.overdue.isNotEmpty()) add("${today.overdue.size} overdue")
                if (today.dueToday.isNotEmpty()) add("${today.dueToday.size} due today")
                if (today.approvals.isNotEmpty()) add("${today.approvals.size} approvals")
                if (today.events.isNotEmpty()) add("${today.events.size} events")
                if (today.waiting.isNotEmpty()) add("${today.waiting.size} waiting for")
            }
            if (parts.isNotEmpty()) row {
                Text(parts.joinToString(" · "), style = Type.footnote, color = Palette.secondary)
            }
        }
        weather?.let { current ->
            row {
                Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(current.text, style = Type.footnote, color = Palette.secondary, modifier = Modifier.weight(1f))
                    LinkButton("Change place") { editingPlace = true }
                }
            }
        }
        if (weather == null || editingPlace) {
            row {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    FieldBox(place, { place = it }, "Weather place (Madrid)")
                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                        SecondaryButton("Save place", enabled = place.isNotBlank()) { savePlace() }
                    }
                }
            }
        }
        if (weatherError != null && weather == null) row {
            Text(weatherError ?: "", style = Type.footnote, color = Palette.tertiary)
        }
        row {
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Box(Modifier.weight(1f))
                SecondaryButton(if (busy) "Reading…" else "Refresh", enabled = !busy) { scope.launch { load() } }
            }
        }
    }
}
