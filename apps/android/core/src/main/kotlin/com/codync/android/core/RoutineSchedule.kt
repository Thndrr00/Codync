package com.codync.android.core

/**
 * Options of the visual routine schedule editor, mirroring the styles the host's schedule editor
 * understands (`host/src/routines/editor.rs`). The host builds the cron expression, validates it and
 * computes the next runs; this only lists what the editor can offer.
 */
object RoutineSchedule {
    /** Trigger kinds the editor edits; any other (`once`, events, several triggers) keeps the original triggers. */
    val editableKinds = listOf("cron", "interval", "webhook")
    /** Style → title, in display order. `custom` is the raw cron expression. */
    val styles = listOf(
        "minutes" to "Every few minutes", "hourly" to "Hourly", "daily" to "Daily", "weekdays" to "Weekdays",
        "weekly" to "Weekly", "days" to "Chosen days", "monthly" to "Monthly", "custom" to "Cron",
    )
    /** Minute steps that divide the hour evenly; the host rejects others. */
    val minuteSteps = listOf(1, 5, 10, 15, 20, 30)
    /** Interval unit in seconds → title; the host accepts exactly these. */
    val intervalUnits = listOf(1L to "Seconds", 60L to "Minutes", 3600L to "Hours", 86400L to "Days")
    /** Weekdays as the host numbers them: 0 Sunday … 6 Saturday. */
    val weekdayInitials = listOf("S", "M", "T", "W", "T", "F", "S")
    val weekdayNames = listOf("Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday")

    /** Styles whose cron has an hour as well as a minute. */
    fun usesTime(style: String): Boolean = style in listOf("daily", "weekdays", "weekly", "days", "monthly")
}
