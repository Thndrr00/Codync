package com.codync.android

import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.glance.GlanceModifier
import androidx.glance.layout.Column
import androidx.glance.layout.Row
import androidx.glance.layout.Spacer
import androidx.glance.layout.fillMaxSize
import androidx.glance.layout.fillMaxWidth
import androidx.glance.layout.height
import androidx.glance.text.FontWeight
import androidx.glance.text.Text
import androidx.glance.text.TextStyle
import androidx.glance.color.ColorProvider
import com.codync.android.core.LockScreenSummary
import com.codync.android.core.WidgetFreshness
import com.codync.android.core.WidgetKind

class LockSummaryWidget : CodyncWidget(WidgetKind.LockSummary)

private val lockInk = ColorProvider(day = Color(0xFF141414), night = Color(0xFFF2F2F2))
private val lockSecondary = ColorProvider(day = Color(0xFF6B6B6B), night = Color(0xFF9A9A9A))

/** Counts and freshness only: this renderer never exposes bot names, messages, or computer details. */
@Composable internal fun LockSummaryCard(summary: LockScreenSummary) {
    Column(GlanceModifier.fillMaxSize()) {
        Text("Codync", style = TextStyle(lockInk, 13.sp, FontWeight.Medium), maxLines = 1)
        val age = when (summary.freshness) {
            WidgetFreshness.NoComputer -> "Not paired"
            WidgetFreshness.NoData -> "No data"
            WidgetFreshness.Stale -> "Saved · ${summary.ageMinutes ?: 0}m ago"
            WidgetFreshness.Fresh -> if (summary.ageMinutes == 0L) "Updated now" else "${summary.ageMinutes ?: 0}m ago"
        }
        Text(age, style = TextStyle(lockSecondary, 10.sp), maxLines = 1)
        Spacer(GlanceModifier.height(8.dp))
        when (summary.freshness) {
            WidgetFreshness.NoComputer -> Text("Pair a computer in Codync.", style = TextStyle(lockSecondary, 12.sp))
            WidgetFreshness.NoData -> Text("Open Codync to refresh status.", style = TextStyle(lockSecondary, 12.sp))
            WidgetFreshness.Fresh, WidgetFreshness.Stale -> Row(GlanceModifier.fillMaxWidth()) {
                Column(GlanceModifier.defaultWeight()) {
                    Text(summary.workingCount.toString(), style = TextStyle(lockInk, 28.sp, FontWeight.Medium))
                    Text("Working", style = TextStyle(lockSecondary, 10.sp), maxLines = 1)
                }
                Column(GlanceModifier.defaultWeight()) {
                    Text(summary.attentionCount.toString(), style = TextStyle(lockInk, 28.sp, FontWeight.Medium))
                    Text("Needs attention", style = TextStyle(lockSecondary, 10.sp), maxLines = 1)
                }
            }
        }
    }
}
