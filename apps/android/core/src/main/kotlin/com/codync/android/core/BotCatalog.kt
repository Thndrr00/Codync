package com.codync.android.core

import kotlinx.serialization.Serializable

/** A ready-made bot from the host's catalog (`botCatalog`). */
@Serializable data class BotProfile(
    val id: String,
    val name: String,
    val summary: String = "",
    val instructions: String = "",
    val avatarColor: String,
    val avatarShape: String,
    /** Offered as a suggestion on the New bot screen. */
    val featured: Boolean = false,
)

/** A group chat led by a coordinator bot, with the catalog bots it hands work to. */
@Serializable data class TeamProfile(
    val id: String,
    val name: String,
    val summary: String = "",
    val purpose: String = "",
    val coordinator: BotProfile,
    /** Ids of catalog bots. */
    val members: List<String> = emptyList(),
)

@Serializable data class BotCatalog(val bots: List<BotProfile> = emptyList(), val teams: List<TeamProfile> = emptyList()) {
    /** The coordinator, then the teammates the catalog has (an unknown id is skipped). */
    fun profiles(team: TeamProfile): List<BotProfile> = listOf(team.coordinator) + team.members.mapNotNull { id -> bots.firstOrNull { it.id == id } }
}

/** A draft that starts from [profile]: its name, look and instructions; the agent and model stay the draft's. */
fun BotDraft.from(profile: BotProfile): BotDraft =
    copy(name = profile.name, description = profile.instructions, avatarColor = profile.avatarColor, avatarShape = profile.avatarShape)
