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
    val routines: List<InsightRoutine> = emptyList(),
    val usage: InsightUsage = InsightUsage(),
    val skills: InsightSkills = InsightSkills(),
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
