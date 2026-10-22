package com.codync.android.core

import kotlinx.serialization.Serializable
import kotlinx.serialization.json.*

/** Schedules execute on the host. Raw triggers retain fields the mobile editor does not edit. */
@Serializable data class Routine(
    val id: String, val botId: String, val name: String, val instruction: String,
    val triggers: List<JsonObject>, val enabled: Boolean,
    val createdAt: Long = 0, val updatedAt: Long = 0,
    val triggerDescriptions: List<String> = emptyList(), val nextRunAt: Long? = null,
    val timeoutSeconds: Int? = null, val lastError: String? = null,
) {
    val keepsOriginal: Boolean get() = triggers.size != 1 || triggers.single()["type"]?.jsonPrimitive?.content !in listOf("cron", "interval", "webhook")
    val hasWebhook: Boolean get() = triggers.any { it["type"]?.jsonPrimitive?.content in listOf("webhook", "event") }
}

@Serializable data class RoutineRun(
    val id: String, val routineId: String, val botId: String, val status: String,
    val createdAt: Long = 0, val finishedAt: Long? = null, val detail: String? = null, val rootId: String? = null,
) {
    val isActive: Boolean get() = status in listOf("pending", "starting", "running", "recovering")
    val label: String get() = when (status) {
        "pending" -> "Queued"; "starting" -> "Starting"; "running" -> "Running"
        "recovering" -> "Resuming after restart"; "failed" -> "Failed"; "interrupted" -> "Interrupted"
        else -> status.replaceFirstChar { it.uppercase() }
    }
}

@Serializable data class RoutineListing(val routines: List<Routine>, val runs: List<RoutineRun>) {
    fun summary(routine: Routine): String {
        runs.firstOrNull { it.routineId == routine.id && it.isActive }?.let { return it.label }
        if (routine.lastError != null) return "Schedule needs attention"
        runs.firstOrNull { it.routineId == routine.id }?.takeIf { it.status in listOf("failed", "interrupted") }
            ?.let { return "Last run ${it.label.lowercase()}" }
        return routine.triggerDescriptions.joinToString(" · ")
    }
}

@Serializable data class RoutineScheduleDraft(
    val kind: String = "cron", val amount: String = "1", val unit: Long = 3600,
    val calendarStyle: String = "daily", val weekday: Int = 1, val selectedDays: List<Int> = emptyList(),
    val monthDay: Int = 1, val minuteStep: Int = 15, val hour: Int = 9, val minute: Int = 0,
    val at: Long = 0, val expression: String = "", val zone: String = "", val original: List<JsonObject> = emptyList(),
) {
    /**
     * What the editor shows: the host's own reading of the trigger. Kinds the editor can't edit (`once`, events,
     * several triggers) keep the original triggers; the visual style the host detected is retained, never forced to `custom`.
     */
    fun forMobileEditor(): RoutineScheduleDraft = copy(kind = if (kind in RoutineSchedule.editableKinds) kind else "keep")

    /** Toggle one weekday (0 Sunday ... 6 Saturday) of the `days` style; the host rejects an empty set. */
    fun toggleDay(day: Int): RoutineScheduleDraft =
        copy(selectedDays = (if (day in selectedDays) selectedDays - day else selectedDays + day).sorted())

}

@Serializable data class RoutineSchedulePreview(
    val draft: RoutineScheduleDraft, val triggers: List<JsonObject>, val summary: String,
    val nextRunAt: Long? = null, val nextRuns: List<Long>? = null, val warning: String? = null,
)

@Serializable data class RoutineWebhook(val url: String? = null, val localUrl: String, val key: String, val connected: Boolean)

data class RoutineDraft(val id: String? = null, val name: String = "", val instruction: String = "",
    val schedule: RoutineScheduleDraft = RoutineScheduleDraft(), val timeoutSeconds: String = "3600") {
    fun body(botId: String): JsonObject = buildJsonObject {
        put("botId", botId); id?.let { put("id", it) }; put("name", name); put("instruction", instruction)
        put("schedule", WireJson.encodeToJsonElement(schedule)); put("timeoutSeconds", timeoutSeconds)
    }
}
