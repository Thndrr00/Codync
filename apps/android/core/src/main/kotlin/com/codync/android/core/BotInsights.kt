package com.codync.android.core

import kotlinx.serialization.Serializable

/** A read-only summary of one bot, computed by the host. */
@Serializable
data class BotInsights(
    val botId: String = "",
    val description: String = "",
    val model: String = "",
    val provider: String? = null,
    val recentWork: List<String> = emptyList(),
    val activity: InsightActivity = InsightActivity(),
    val health: InsightHealth = InsightHealth(),
    val profile: InsightProfile = InsightProfile(),
    val week: InsightWeek = InsightWeek(),
    val routineSummary: InsightRoutineSummary = InsightRoutineSummary(),
    val routines: List<InsightRoutine> = emptyList(),
    val usage: InsightUsage = InsightUsage(),
    val skills: InsightSkills = InsightSkills(),
)

/** A host-computed summary of one bot's routines. */
@Serializable
data class InsightRoutineSummary(
    val count: String = "",
    val active: Int = 0,
    val paused: Int = 0,
    val scheduled: Int = 0,
    val activePercent: Int = 0,
    val bar: String = "",
    val next: String = "",
    val mix: String = "",
    val clock: Boolean = false,
    val event: Boolean = false,
    val allScheduled: Boolean = false,
    val headline: String = "",
    val tone: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
    val soonest: List<InsightSoonestRoutine> = emptyList(),
    val pausedList: List<InsightPausedRoutine> = emptyList(),
)

@Serializable data class InsightSummaryRow(val key: String = "", val value: String = "")
@Serializable data class InsightSoonestRoutine(
    val id: String = "", val name: String = "", val enabled: Boolean = false,
    val trigger: String = "", val triggerIcon: String = "", val nextIn: String = "",
    val stateIcon: String = "", val stateTone: String = "",
)
@Serializable data class InsightPausedRoutine(val id: String = "", val name: String = "")

/** A host-computed weekly summary for one bot. */
@Serializable
data class InsightWeek(
    val headline: String = "",
    val tone: String = "",
    val verdict: String = "",
    val total: Int = 0,
    val totalLabel: String = "",
    val average: Int = 0,
    val peak: InsightPeak? = null,
    val quietDays: Int = 0,
    val activeStreak: Int = 0,
    val quietStreak: Int = 0,
    val aboveAverage: Int = 0,
    val change: Int = 0,
    val perfectAttendance: Boolean = false,
    val grew: Boolean = false,
    val trend: InsightTrend = InsightTrend(),
    val top: List<InsightDayBar> = emptyList(),
    val quiet: List<InsightDayBar> = emptyList(),
    val days: List<InsightDayCount> = emptyList(),
)

@Serializable data class InsightPeak(val label: String = "", val count: Int = 0)
@Serializable data class InsightTrend(val kind: String = "", val label: String = "", val icon: String = "", val tone: String = "")
@Serializable data class InsightDayBar(val label: String = "", val count: Int = 0, val bar: String = "")
@Serializable data class InsightDayCount(val key: String = "", val label: String = "", val count: Int = 0)

/** A host-computed capability profile for one bot. */
@Serializable
data class InsightProfile(
    val headline: String = "",
    val enabled: Int = 0,
    val total: Int = 0,
    val missing: Int = 0,
    val coverage: Int = 0,
    val bar: String = "",
    val nextUp: List<InsightNextUp> = emptyList(),
    val caps: List<InsightCapability> = emptyList(),
)

@Serializable data class InsightNextUp(val title: String = "", val icon: String = "")
@Serializable data class InsightCapability(
    val id: String = "", val title: String = "", val detail: String = "",
    val icon: String = "", val tone: String = "", val enabled: Boolean = false,
    val stateIcon: String = "", val stateTone: String = "",
)

/** A host-computed configuration health report for one bot. */
@Serializable
data class InsightHealth(
    val score: Int = 0,
    val label: String = "",
    val tone: String = "",
    val headline: String = "",
    val blocking: Boolean = false,
    val errors: Int = 0,
    val warnings: Int = 0,
    val counts: List<InsightSeverityCount> = emptyList(),
    val checks: List<InsightCheck> = emptyList(),
)

@Serializable data class InsightSeverityCount(val severity: String = "", val count: Int = 0)
@Serializable data class InsightCheck(
    val id: String = "", val title: String = "", val detail: String = "",
    val severity: String = "", val tone: String = "", val icon: String = "",
)

/** A host-computed activity timeline: folded runs, a daily strip and a summary. */
@Serializable
data class InsightActivity(
    val headline: String = "",
    val steps: Int = 0,
    val errors: Int = 0,
    val dominant: String = "",
    val span: String = "",
    val total: Int = 0,
    val counts: List<InsightCount> = emptyList(),
    val groups: List<InsightGroup> = emptyList(),
    val items: List<InsightStep> = emptyList(),
    val days: List<InsightDay> = emptyList(),
)

@Serializable data class InsightCount(val kind: String = "", val count: Int = 0, val percent: Int = 0)
@Serializable data class InsightGroup(val kind: String = "", val label: String = "", val icon: String = "", val tone: String = "", val count: Int = 0)
@Serializable data class InsightStep(val text: String = "", val detail: String = "", val status: String = "", val icon: String = "", val tone: String = "")
@Serializable data class InsightDay(val day: String = "", val label: String = "", val count: Int = 0, val width: Int = 0)

@Serializable
data class InsightRoutine(
    val id: String = "",
    val name: String = "",
    val enabled: Boolean = true,
    val schedules: List<String> = emptyList(),
    val nextIn: String? = null,
    val nextInMs: Long? = null,
)

@Serializable
data class InsightUsage(
    val used: Long = 0L,
    val usedText: String = "",
    val limit: Long? = null,
    val resetsAt: Long? = null,
)

@Serializable
data class InsightSkills(
    val total: Int = 0,
    val resources: Map<String, Int> = emptyMap(),
)
