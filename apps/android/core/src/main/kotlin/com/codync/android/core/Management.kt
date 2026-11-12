package com.codync.android.core

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.*

/** A group uses the existing bot endpoints; it never owns an agent or workspace. */
data class GroupDraft(
    val id: String? = null,
    val name: String = "",
    val description: String = "",
    val members: List<String> = emptyList(),
    val pinned: Boolean = false,
    val chiefBotId: String? = null,
    val team: TeamSettings? = null,
) {
    fun body(roster: List<Bot>): JsonObject {
        val picked = members.distinct().map { id ->
            requireNotNull(roster.firstOrNull { it.id == id && !it.deleted && it.kind != "group" }) {
                "A member is no longer available. Remove it before saving."
            }
        }
        require(picked.isNotEmpty()) { "Choose at least one bot." }
        return buildJsonObject {
            id?.let { put("id", it) }
            put("kind", "group")
            put("name", name.trim().ifEmpty { picked.joinToString(", ") { it.name } })
            put("description", description.trim())
            put("members", JsonArray(picked.map { JsonPrimitive(it.id) }))
            put("pinned", pinned)
            put("chiefBotId", chiefBotId?.let { JsonPrimitive(it) } ?: JsonNull)
            put("team", team?.let { settings ->
                buildJsonObject {
                    put("mode", settings.mode)
                    put("voice", settings.voice)
                    put("members", JsonArray(settings.members.map { member ->
                        buildJsonObject {
                            put("botId", member.botId)
                            put("role", member.role)
                            put("responsibility", member.responsibility)
                            put("reportsTo", member.reportsTo?.let { JsonPrimitive(it) } ?: JsonNull)
                            put("x", member.x)
                            put("y", member.y)
                        }
                    }))
                }
            } ?: JsonNull)
        }
    }

    companion object {
        fun from(bot: Bot): GroupDraft {
            require(bot.kind == "group")
            return GroupDraft(bot.id, bot.name, bot.description, bot.members, bot.pinned, bot.chiefBotId, bot.team)
        }
    }
}
