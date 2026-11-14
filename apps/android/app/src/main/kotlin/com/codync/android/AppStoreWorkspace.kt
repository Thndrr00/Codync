package com.codync.android

import kotlinx.serialization.json.*

/** One card on the board (host/store/board.rs). */
internal data class BoardTask(
    val id: String,
    val title: String,
    val detail: String,
    val status: String,
    val botId: String?,
    val createdAt: Long,
    val updatedAt: Long,
) {
    val isDone get() = status == "done"

    companion object {
        val statuses = listOf("todo", "doing", "waiting", "done")
    }
}

/** A group saved as a reusable team. */
internal data class TeamTemplate(
    val id: String,
    val name: String,
    val description: String,
    val members: List<String>,
)

/** A page this computer serves to anyone with the link. */
internal data class SharedLink(
    val token: String,
    val title: String,
    val url: String,
    val expiresAt: Long?,
)

private fun JsonObject.str(key: String): String? = (this[key] as? JsonPrimitive)?.takeIf { it.isString }?.content
private fun JsonObject.num(key: String): Long? = (this[key] as? JsonPrimitive)?.longOrNull
private fun JsonObject.bool(key: String): Boolean? = (this[key] as? JsonPrimitive)?.booleanOrNull

private fun taskOf(json: JsonObject): BoardTask = BoardTask(
    id = json.str("id") ?: "",
    title = json.str("title") ?: "",
    detail = json.str("detail") ?: "",
    status = json.str("status") ?: "todo",
    botId = json.str("botId"),
    createdAt = json.num("createdAt") ?: 0,
    updatedAt = json.num("updatedAt") ?: 0,
)

private fun templateOf(json: JsonObject): TeamTemplate = TeamTemplate(
    id = json.str("id") ?: "",
    name = json.str("name") ?: "",
    description = json.str("description") ?: "",
    members = (json["members"] as? JsonArray)?.mapNotNull { (it as? JsonPrimitive)?.content } ?: emptyList(),
)

private fun linkOf(json: JsonObject): SharedLink = SharedLink(
    token = json.str("token") ?: "",
    title = json.str("title") ?: "Codync",
    url = json.str("url") ?: "",
    expiresAt = json.num("expiresAt"),
)

internal suspend fun AppStore.boardTasks(): List<BoardTask> =
    (hostCall("boardTasks", buildJsonObject {}).jsonObject["tasks"] as? JsonArray)
        ?.mapNotNull { (it as? JsonObject)?.let(::taskOf) } ?: emptyList()

internal suspend fun AppStore.saveBoardTask(title: String, detail: String = ""): BoardTask =
    taskOf(hostCall("saveBoardTask", buildJsonObject {
        put("title", title)
        put("detail", detail)
        put("status", "todo")
    }).jsonObject["task"]?.jsonObject ?: buildJsonObject {})

internal suspend fun AppStore.moveBoardTask(id: String, status: String): Boolean =
    hostCall("moveBoardTask", buildJsonObject { put("id", id); put("status", status) })
        .jsonObject["moved"]?.jsonPrimitive?.booleanOrNull == true

internal suspend fun AppStore.deleteBoardTask(id: String): Boolean =
    hostCall("deleteBoardTask", buildJsonObject { put("id", id) })
        .jsonObject["deleted"]?.jsonPrimitive?.booleanOrNull == true

internal suspend fun AppStore.teamTemplates(): List<TeamTemplate> =
    (hostCall("teamTemplates", buildJsonObject {}).jsonObject["templates"] as? JsonArray)
        ?.mapNotNull { (it as? JsonObject)?.let(::templateOf) } ?: emptyList()

internal suspend fun AppStore.installTeamTemplate(id: String): Boolean {
    hostCall("installTeamTemplate", buildJsonObject { put("id", id) })
    return true
}

internal suspend fun AppStore.sharedLinks(): List<SharedLink> =
    (hostCall("sharedLinks", buildJsonObject {}).jsonObject["links"] as? JsonArray)
        ?.mapNotNull { (it as? JsonObject)?.let(::linkOf) } ?: emptyList()

internal suspend fun AppStore.revokeSharedLink(token: String): Boolean =
    hostCall("revokeShare", buildJsonObject { put("token", token) })
        .jsonObject["revoked"]?.jsonPrimitive?.booleanOrNull == true

/** One group member's turn in a short meeting; `audio` is base64 for `audioFormat`. */
internal data class MeetingTurn(
    val botId: String,
    val name: String,
    val text: String,
    val audio: String?,
    val audioFormat: String?,
)

internal data class TeamMeeting(val topic: String, val turns: List<MeetingTurn>, val minutes: String)

internal suspend fun AppStore.teamMeeting(groupId: String, topic: String, voice: Boolean = false): TeamMeeting {
    val answer = hostCall("teamMeeting", buildJsonObject {
        put("botId", groupId)
        put("topic", topic)
        put("voice", voice)
    }, 240_000).jsonObject
    val turns = (answer["turns"] as? JsonArray)?.mapNotNull { turn ->
        val json = turn as? JsonObject ?: return@mapNotNull null
        MeetingTurn(
            botId = json.str("botId") ?: "",
            name = json.str("name") ?: "",
            text = json.str("text") ?: "",
            audio = json.str("audio"),
            audioFormat = json.str("audioFormat"),
        )
    } ?: emptyList()
    return TeamMeeting(answer.str("topic") ?: topic, turns, answer.str("minutes") ?: "")
}
