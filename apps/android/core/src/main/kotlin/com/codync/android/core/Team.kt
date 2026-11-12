package com.codync.android.core

import kotlinx.serialization.Serializable

/** One member on a group's team chart: its role and who it reports to. */
@Serializable
data class TeamMember(
    val botId: String,
    val role: String = "",
    val responsibility: String = "",
    val reportsTo: String? = null,
    val x: Double = 0.0,
    val y: Double = 0.0,
)

/** How a group works as a team: one voice through its lead, or the classic round table. */
@Serializable
data class TeamSettings(
    /** `team` answers as one voice through its lead; `roundtable` keeps every member answering. */
    val mode: String = "smart",
    /** `lead`, `lead_and_hands` (a member the lead names may also speak) or `all`. */
    val voice: String = "lead",
    /** One entry per member on the chart. */
    val members: List<TeamMember> = emptyList(),
) {
    /** The chart in step with the group: every member present, the lead at the root. */
    fun reconciled(members: List<String>, lead: String?): TeamSettings {
        val root = lead?.takeIf { it in members } ?: members.firstOrNull()
        val kept = this.members.filter { it.botId in members }
        val missing = members.filter { id -> kept.none { it.botId == id } }
        val rows = (kept + missing.map { TeamMember(botId = it) }).map { row ->
            when {
                row.botId == root -> row.copy(reportsTo = null)
                row.reportsTo == null || row.reportsTo !in members -> row.copy(reportsTo = root)
                else -> row
            }
        }
        return copy(members = rows)
    }
}
