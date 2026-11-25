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
    val library: InsightLibrary = InsightLibrary(),
    val trust: InsightTrust = InsightTrust(),
    val resilience: InsightResilience = InsightResilience(),
    val identity: InsightIdentity = InsightIdentity(),
    val connectors: InsightConnectors = InsightConnectors(),
    val tuning: InsightTuning = InsightTuning(),
    val memory: InsightMemory = InsightMemory(),
    val workspace: InsightWorkspace = InsightWorkspace(),
    val session: InsightSession = InsightSession(),
    val advice: InsightAdvice = InsightAdvice(),
    val digest: InsightDigest = InsightDigest(),
    val timezone: InsightTimezone = InsightTimezone(),
    val approvals: InsightApprovals = InsightApprovals(),
    val routines: List<InsightRoutine> = emptyList(),
    val usage: InsightUsage = InsightUsage(),
    val skills: InsightSkills = InsightSkills(),
)

/** A host-computed summary of approval cards waiting on the user. */
@Serializable
data class InsightApprovals(
    val headline: String = "",
    val tone: String = "",
    val total: String = "",
    val waiting: Int = 0,
    val urgent: Int = 0,
    val oldest: String = "",
    val bar: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
    val items: List<InsightApprovalItem> = emptyList(),
)

@Serializable data class InsightApprovalItem(
    val id: String = "", val title: String = "", val detail: String = "",
    val age: String = "", val icon: String = "", val tone: String = "", val urgent: Boolean = false,
)

/** A host-computed summary of the time zones a bot's routines use. */
@Serializable
data class InsightTimezone(
    val headline: String = "",
    val tone: String = "",
    val local: String = "",
    val primary: String = "",
    val zones: Int = 0,
    val schedules: String = "",
    val multi: Boolean = false,
    val remote: Boolean = false,
    val summary: List<InsightSummaryRow> = emptyList(),
    val all: List<InsightZoneLine> = emptyList(),
    val remoteList: List<InsightRemoteZone> = emptyList(),
)

@Serializable data class InsightZoneLine(
    val name: String = "", val label: String = "", val icon: String = "",
    val tone: String = "", val count: Int = 0, val local: Boolean = false,
    val percent: Int = 0, val bar: String = "",
)
@Serializable data class InsightRemoteZone(val label: String = "", val count: Int = 0)

/** A host-computed executive digest for one bot. */
@Serializable
data class InsightDigest(
    val headline: String = "",
    val oneLine: String = "",
    val label: String = "",
    val tone: String = "",
    val score: Int = 0,
    val areas: String = "",
    val onTrack: Int = 0,
    val attention: Int = 0,
    val summary: List<InsightSummaryRow> = emptyList(),
    val highlights: List<InsightDigestStat> = emptyList(),
    val stats: List<InsightDigestStat> = emptyList(),
)

@Serializable data class InsightDigestStat(
    val id: String = "", val label: String = "", val value: String = "",
    val icon: String = "", val tone: String = "",
)

/** A host-computed, prioritised suggestion list for one bot. */
@Serializable
data class InsightAdvice(
    val headline: String = "",
    val label: String = "",
    val tone: String = "",
    val score: Int = 0,
    val total: String = "",
    val urgent: Int = 0,
    val summary: List<InsightSummaryRow> = emptyList(),
    val items: List<InsightAdviceItem> = emptyList(),
)

@Serializable data class InsightAdviceItem(
    val id: String = "", val title: String = "", val detail: String = "",
    val icon: String = "", val tone: String = "", val priority: Int = 0,
    val priorityLabel: String = "", val priorityIcon: String = "", val priorityTone: String = "",
)

/** A host-computed session and sync state for one bot. */
@Serializable
data class InsightSession(
    val headline: String = "",
    val label: String = "",
    val tone: String = "",
    val score: Int = 0,
    val session: String = "",
    val sessionIcon: String = "",
    val sessionTone: String = "",
    val revision: String = "",
    val unread: String = "",
    val bar: String = "",
    val deleted: Boolean = false,
    val summary: List<InsightSummaryRow> = emptyList(),
    val lines: List<InsightSessionLine> = emptyList(),
)

@Serializable data class InsightSessionLine(
    val id: String = "", val label: String = "", val detail: String = "",
    val icon: String = "", val tone: String = "", val ok: Boolean = false,
    val stateIcon: String = "", val stateTone: String = "",
)

/** A host-computed summary of one bot's work environment. */
@Serializable
data class InsightWorkspace(
    val headline: String = "",
    val label: String = "",
    val tone: String = "",
    val score: Int = 0,
    val folder: String = "",
    val folderIcon: String = "",
    val folderTone: String = "",
    val set: Int = 0,
    val summary: List<InsightSummaryRow> = emptyList(),
    val lines: List<InsightWorkspaceLine> = emptyList(),
)

@Serializable data class InsightWorkspaceLine(
    val id: String = "", val label: String = "", val detail: String = "",
    val icon: String = "", val tone: String = "", val set: Boolean = false,
    val stateIcon: String = "", val stateTone: String = "",
)

/** A host-computed memory posture for one bot. */
@Serializable
data class InsightMemory(
    val headline: String = "",
    val label: String = "",
    val tone: String = "",
    val score: Int = 0,
    val shared: String = "",
    val sharedIcon: String = "",
    val sharedTone: String = "",
    val sharedDetail: String = "",
    val harness: Boolean = false,
    val harnessIcon: String = "",
    val harnessTone: String = "",
    val skills: Int = 0,
    val on: Int = 0,
    val hasAny: Boolean = false,
    val summary: List<InsightSummaryRow> = emptyList(),
    val lines: List<InsightMemoryLine> = emptyList(),
)

@Serializable data class InsightMemoryLine(
    val id: String = "", val title: String = "", val detail: String = "",
    val icon: String = "", val tone: String = "", val on: Boolean = false,
    val stateIcon: String = "", val stateTone: String = "",
)

/** A host-computed summary of one bot's model tuning. */
@Serializable
data class InsightTuning(
    val headline: String = "",
    val tone: String = "",
    val count: String = "",
    val total: Int = 0,
    val on: Int = 0,
    val off: Int = 0,
    val fast: Boolean = false,
    val effort: String = "",
    val effortIcon: String = "",
    val reasoning: Boolean = false,
    val summary: List<InsightSummaryRow> = emptyList(),
    val options: List<InsightTuningOption> = emptyList(),
    val enabled: List<InsightTuningEnabled> = emptyList(),
)

@Serializable data class InsightTuningOption(
    val key: String = "", val title: String = "", val value: String = "",
    val icon: String = "", val tone: String = "",
)
@Serializable data class InsightTuningEnabled(val title: String = "", val value: String = "", val icon: String = "")

/** A host-computed summary of one bot's connected accounts. */
@Serializable
data class InsightConnectors(
    val headline: String = "",
    val tone: String = "",
    val total: Int = 0,
    val distinct: Int = 0,
    val duplicates: List<String> = emptyList(),
    val chat: Boolean = false,
    val mail: Boolean = false,
    val code: Boolean = false,
    val money: Boolean = false,
    val summary: List<InsightSummaryRow> = emptyList(),
    val categories: List<InsightConnectorCategory> = emptyList(),
    val connectors: List<InsightConnectorLine> = emptyList(),
)

@Serializable data class InsightConnectorCategory(
    val kind: String = "", val count: Int = 0, val icon: String = "",
    val tone: String = "", val percent: Int = 0, val bar: String = "",
)
@Serializable data class InsightConnectorLine(
    val name: String = "", val category: String = "", val icon: String = "",
    val tone: String = "", val detail: String = "",
)

/** A host-computed roster identity for one bot. */
@Serializable
data class InsightIdentity(
    val name: String = "",
    val headline: String = "",
    val tone: String = "",
    val band: String = "",
    val bandIcon: String = "",
    val bandTone: String = "",
    val section: String = "",
    val mode: String = "",
    val rank: Int = 0,
    val color: String = "",
    val colorIcon: String = "",
    val shape: String = "",
    val look: String = "",
    val visible: Boolean = false,
    val score: Int = 0,
    val count: String = "",
    val summary: List<InsightSummaryRow> = emptyList(),
    val traits: List<InsightIdentityTrait> = emptyList(),
)

@Serializable data class InsightIdentityTrait(
    val id: String = "", val label: String = "", val detail: String = "",
    val icon: String = "", val tone: String = "", val on: Boolean = false,
    val stateIcon: String = "", val stateTone: String = "",
)

/** A host-computed resilience summary for one bot. */
@Serializable
data class InsightResilience(
    val headline: String = "",
    val label: String = "",
    val tone: String = "",
    val score: Int = 0,
    val bar: String = "",
    val chain: String = "",
    val total: Int = 0,
    val hasChain: Boolean = false,
    val hasOverride: Boolean = false,
    val customCommands: Int = 0,
    val selfFallbacks: Boolean = false,
    val switches: List<InsightSwitch> = emptyList(),
    val summary: List<InsightSummaryRow> = emptyList(),
    val fallbacks: List<InsightFallback> = emptyList(),
)

@Serializable data class InsightSwitch(val label: String = "", val on: Boolean = false)
@Serializable data class InsightFallback(
    val backend: String = "", val model: String = "", val command: String = "",
    val label: String = "", val icon: String = "", val tone: String = "",
)

/** A host-computed trust posture for one bot. */
@Serializable
data class InsightTrust(
    val mode: String = "",
    val modeLabel: String = "",
    val modeIcon: String = "",
    val modeTone: String = "",
    val modeDetail: String = "",
    val score: Int = 0,
    val tone: String = "",
    val label: String = "",
    val headline: String = "",
    val total: Int = 0,
    val countLabel: String = "",
    val broad: Int = 0,
    val command: Int = 0,
    val hasDeny: Boolean = false,
    val hasBroadAllow: Boolean = false,
    val asksForEverything: Boolean = false,
    val decisions: List<InsightDecisionCount> = emptyList(),
    val summary: List<InsightSummaryRow> = emptyList(),
    val rules: List<InsightTrustRule> = emptyList(),
)

@Serializable data class InsightDecisionCount(val decision: String = "", val count: Int = 0)
@Serializable data class InsightTrustRule(
    val pattern: String = "", val kind: String = "", val decision: String = "",
    val icon: String = "", val tone: String = "", val broad: Boolean = false,
)

/** A host-computed summary of one bot's skill library. */
@Serializable
data class InsightLibrary(
    val headline: String = "",
    val tone: String = "",
    val count: String = "",
    val total: Int = 0,
    val distinct: Int = 0,
    val duplicates: List<String> = emptyList(),
    val hasScript: Boolean = false,
    val hasReference: Boolean = false,
    val hasAsset: Boolean = false,
    val summary: List<InsightSummaryRow> = emptyList(),
    val kinds: List<InsightLibraryKind> = emptyList(),
    val skills: List<InsightLibrarySkill> = emptyList(),
)

@Serializable data class InsightLibraryKind(
    val kind: String = "", val count: Int = 0, val icon: String = "",
    val tone: String = "", val percent: Int = 0, val bar: String = "",
)
@Serializable data class InsightLibrarySkill(
    val name: String = "", val kind: String = "", val icon: String = "",
    val tone: String = "", val detail: String = "",
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
