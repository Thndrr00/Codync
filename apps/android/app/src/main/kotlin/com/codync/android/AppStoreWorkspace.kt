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

/** Something other people owe the person; kept on the computer and repeated in the daily digest. */
internal data class WaitingItem(
    val id: String,
    val title: String,
    val counterpart: String,
    val url: String,
    val note: String,
    val dueAt: Long?,
    val done: Boolean,
)

private fun waitingOf(json: JsonObject): WaitingItem = WaitingItem(
    id = json.str("id") ?: "",
    title = json.str("title") ?: "",
    counterpart = json.str("counterpart") ?: "",
    url = json.str("url") ?: "",
    note = json.str("note") ?: "",
    dueAt = json.num("dueAt"),
    done = json.bool("done") ?: false,
)

internal suspend fun AppStore.waitingItems(): List<WaitingItem> =
    (hostCall("waitingItems", buildJsonObject {}).jsonObject["items"] as? JsonArray)
        ?.mapNotNull { (it as? JsonObject)?.let(::waitingOf) } ?: emptyList()

internal suspend fun AppStore.saveWaitingItem(title: String, counterpart: String, dueAt: Long?): WaitingItem =
    waitingOf(hostCall("saveWaitingItem", buildJsonObject {
        put("title", title)
        put("counterpart", counterpart)
        if (dueAt != null) put("dueAt", dueAt)
    }).jsonObject["item"]?.jsonObject ?: buildJsonObject {})

internal suspend fun AppStore.setWaitingDone(id: String, done: Boolean): Boolean =
    hostCall("setWaitingDone", buildJsonObject { put("id", id); put("done", done) })
        .jsonObject["item"]?.jsonObject?.let { it.bool("done") } == true

internal suspend fun AppStore.deleteWaitingItem(id: String): Boolean =
    hostCall("deleteWaitingItem", buildJsonObject { put("id", id) })
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

/** A read-only calendar feed this computer subscribes to. */
internal data class CalendarFeed(val id: String, val name: String, val url: String, val fetchedAt: Long?)

/** One event from a subscribed calendar; times are Unix milliseconds. */
internal data class CalendarEvent(
    val feed: String,
    val title: String,
    val start: Long,
    val end: Long?,
    val allDay: Boolean,
    val location: String?,
    val url: String?,
)

private fun feedOf(json: JsonObject): CalendarFeed = CalendarFeed(
    id = json.str("id") ?: "",
    name = json.str("name") ?: "Calendar",
    url = json.str("url") ?: "",
    fetchedAt = json.num("fetchedAt"),
)

private fun eventOf(json: JsonObject): CalendarEvent = CalendarEvent(
    feed = json.str("feed") ?: "",
    title = json.str("title") ?: "Untitled",
    start = json.num("start") ?: 0,
    end = json.num("end"),
    allDay = json.bool("allDay") ?: false,
    location = json.str("location"),
    url = json.str("url"),
)

internal suspend fun AppStore.calendarFeeds(): List<CalendarFeed> =
    (hostCall("calendarFeeds", buildJsonObject {}).jsonObject["feeds"] as? JsonArray)
        ?.mapNotNull { (it as? JsonObject)?.let(::feedOf) } ?: emptyList()

internal suspend fun AppStore.addCalendarFeed(name: String, url: String): CalendarFeed =
    feedOf(hostCall("addCalendarFeed", buildJsonObject {
        put("name", name)
        put("url", url)
    }, 60_000).jsonObject["feed"]?.jsonObject ?: buildJsonObject {})

internal suspend fun AppStore.removeCalendarFeed(id: String): Boolean =
    hostCall("removeCalendarFeed", buildJsonObject { put("id", id) })
        .jsonObject["removed"]?.jsonPrimitive?.booleanOrNull == true

internal suspend fun AppStore.calendarAgenda(days: Int = 14): List<CalendarEvent> =
    (hostCall("calendarAgenda", buildJsonObject { put("days", days) }).jsonObject["events"] as? JsonArray)
        ?.mapNotNull { (it as? JsonObject)?.let(::eventOf) } ?: emptyList()

/** An email account on this computer; `configured` says its password is saved. */
internal data class EmailAccount(
    val id: String,
    val name: String,
    val address: String,
    val imapHost: String,
    val smtpHost: String,
    val configured: Boolean,
)

/** One message header from the inbox. */
internal data class EmailMessage(val uid: String, val from: String, val subject: String, val date: String)

private fun mailAccountOf(json: JsonObject): EmailAccount = EmailAccount(
    id = json.str("id") ?: "",
    name = json.str("name") ?: "Email",
    address = json.str("address") ?: "",
    imapHost = json.str("imapHost") ?: "",
    smtpHost = json.str("smtpHost") ?: "",
    configured = json.bool("configured") ?: false,
)

private fun mailOf(json: JsonObject): EmailMessage = EmailMessage(
    uid = json.str("uid") ?: "",
    from = json.str("from") ?: "",
    subject = json.str("subject") ?: "",
    date = json.str("date") ?: "",
)

internal suspend fun AppStore.emailAccounts(): List<EmailAccount> =
    (hostCall("emailAccounts", buildJsonObject {}).jsonObject["accounts"] as? JsonArray)
        ?.mapNotNull { (it as? JsonObject)?.let(::mailAccountOf) } ?: emptyList()

internal suspend fun AppStore.saveEmailAccount(name: String, address: String, password: String, imapHost: String, smtpHost: String, username: String): EmailAccount =
    mailAccountOf(hostCall("saveEmailAccount", buildJsonObject {
        put("name", name)
        put("address", address)
        put("password", password)
        put("imapHost", imapHost)
        put("smtpHost", smtpHost)
        put("username", username)
    }).jsonObject["account"]?.jsonObject ?: buildJsonObject {})

internal suspend fun AppStore.deleteEmailAccount(id: String): Boolean =
    hostCall("deleteEmailAccount", buildJsonObject { put("id", id) })
        .jsonObject["deleted"]?.jsonPrimitive?.booleanOrNull == true

internal suspend fun AppStore.emailInbox(account: String, limit: Int = 20): List<EmailMessage> =
    (hostCall("emailInbox", buildJsonObject { put("account", account); put("limit", limit) }, 60_000)
        .jsonObject["messages"] as? JsonArray)
        ?.mapNotNull { (it as? JsonObject)?.let(::mailOf) } ?: emptyList()

internal suspend fun AppStore.sendEmail(account: String, to: String, subject: String, body: String): Boolean =
    hostCall("emailSend", buildJsonObject {
        put("account", account)
        put("to", to)
        put("subject", subject)
        put("body", body)
    }, 60_000).jsonObject["sent"]?.jsonPrimitive?.booleanOrNull == true

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
