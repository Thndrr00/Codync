package com.codync.android

import java.util.Base64
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

/** A meeting note as a list shows it. */
internal data class MeetingSummary(
    val id: String,
    val title: String,
    val startedAt: Long,
    val finishedAt: Long,
    val chars: Int,
    val hasMinutes: Boolean,
)

/** One meeting's transcript and, once asked for, its minutes. */
internal data class MeetingNote(
    val id: String,
    val title: String,
    val startedAt: Long,
    val transcript: String,
    val minutes: String?,
    val hasMinutes: Boolean,
)

/** A recording that just started on the computer. */
internal data class MeetingStart(val id: String, val title: String, val startedAt: Long)

private fun meetingSummaryOf(json: JsonObject): MeetingSummary = MeetingSummary(
    id = json.str("id") ?: "",
    title = json.str("title") ?: "Meeting",
    startedAt = json.num("startedAt") ?: 0,
    finishedAt = json.num("finishedAt") ?: 0,
    chars = (json.num("chars") ?: 0).toInt(),
    hasMinutes = json.bool("hasMinutes") ?: false,
)

private fun meetingNoteOf(json: JsonObject): MeetingNote = MeetingNote(
    id = json.str("id") ?: "",
    title = json.str("title") ?: "Meeting",
    startedAt = json.num("startedAt") ?: 0,
    transcript = json.str("transcript") ?: "",
    minutes = json.str("minutes"),
    hasMinutes = json.bool("hasMinutes") ?: !json.str("minutes").isNullOrEmpty(),
)

internal suspend fun AppStore.meetingNotes(): List<MeetingSummary> =
    (hostCall("meetingNotes", buildJsonObject {}).jsonObject["notes"] as? JsonArray)
        ?.mapNotNull { (it as? JsonObject)?.let(::meetingSummaryOf) } ?: emptyList()

internal suspend fun AppStore.meetingNote(id: String): MeetingNote =
    meetingNoteOf(hostCall("meetingNote", buildJsonObject { put("id", id) })
        .jsonObject["note"]?.jsonObject ?: buildJsonObject {})

internal suspend fun AppStore.startMeeting(title: String): MeetingStart {
    val json = hostCall("startMeeting", buildJsonObject { put("title", title) }).jsonObject
    return MeetingStart(
        id = json.str("id") ?: "",
        title = json.str("title") ?: title,
        startedAt = json.num("startedAt") ?: 0,
    )
}

internal suspend fun AppStore.appendMeetingAudio(id: String, pcm: ByteArray) {
    hostCall("appendMeetingAudio", buildJsonObject {
        put("id", id)
        put("audio", Base64.getEncoder().encodeToString(pcm))
    }, 120_000)
}

internal suspend fun AppStore.finishMeeting(id: String): MeetingNote =
    meetingNoteOf(hostCall("finishMeeting", buildJsonObject { put("id", id) }, 600_000)
        .jsonObject["note"]?.jsonObject ?: buildJsonObject {})

internal suspend fun AppStore.cancelMeeting(id: String): Boolean =
    hostCall("cancelMeeting", buildJsonObject { put("id", id) })
        .jsonObject["cancelled"]?.jsonPrimitive?.booleanOrNull == true

internal suspend fun AppStore.meetingMinutes(id: String): MeetingNote =
    meetingNoteOf(hostCall("meetingMinutes", buildJsonObject { put("id", id) }, 600_000)
        .jsonObject["note"]?.jsonObject ?: buildJsonObject {})

internal suspend fun AppStore.deleteMeetingNote(id: String): Boolean =
    hostCall("deleteMeetingNote", buildJsonObject { put("id", id) })
        .jsonObject["deleted"]?.jsonPrimitive?.booleanOrNull == true

/** One encrypted backup already in the person's WebDAV folder. */
internal data class CloudBackupFile(val name: String, val at: Long)

/** Where the computer puts its encrypted backups and how the last one went. */
internal data class CloudBackupStatus(
    val url: String,
    val username: String,
    val enabled: Boolean,
    val keep: Int,
    val lastAt: Long?,
    val lastError: String?,
    val configured: Boolean,
    val hasPassword: Boolean,
    val files: List<CloudBackupFile>,
)

internal data class CloudBackupUpload(val name: String, val bytes: Long)

private fun backupStatusOf(json: JsonObject): CloudBackupStatus = CloudBackupStatus(
    url = json.str("url") ?: "",
    username = json.str("username") ?: "",
    enabled = json.bool("enabled") ?: false,
    keep = (json.num("keep") ?: 14).toInt(),
    lastAt = json.num("lastAt"),
    lastError = json.str("lastError"),
    configured = json.bool("configured") ?: false,
    hasPassword = json.bool("hasPassword") ?: false,
    files = (json["files"] as? JsonArray)?.mapNotNull { (it as? JsonObject)?.let { file ->
        CloudBackupFile(file.str("name") ?: "", file.num("at") ?: 0)
    } } ?: emptyList(),
)

internal suspend fun AppStore.cloudBackupStatus(): CloudBackupStatus =
    backupStatusOf(hostCall("cloudBackupStatus", buildJsonObject {}).jsonObject)

internal suspend fun AppStore.saveCloudBackup(url: String, username: String, password: String, enabled: Boolean, keep: Int): CloudBackupStatus =
    backupStatusOf(hostCall("saveCloudBackup", buildJsonObject {
        put("url", url)
        put("username", username)
        put("password", password)
        put("enabled", enabled)
        put("keep", keep)
    }, 120_000).jsonObject)

internal suspend fun AppStore.runCloudBackup(): CloudBackupUpload {
    val json = hostCall("runCloudBackup", buildJsonObject {}, 300_000).jsonObject
    return CloudBackupUpload(json.str("name") ?: "", json.num("bytes") ?: 0)
}

/** The host's brief for today: board, approvals, calendar and waiting items; no model call. */
internal data class DailyDigest(
    val text: String,
    val overdue: List<String>,
    val dueToday: List<String>,
    val approvals: List<String>,
    val events: List<String>,
    val waiting: List<String>,
)

private fun digestOf(json: JsonObject): DailyDigest {
    fun strings(key: String): List<String> =
        (json[key] as? JsonArray)?.mapNotNull { (it as? JsonPrimitive)?.takeIf { p -> p.isString }?.content } ?: emptyList()
    return DailyDigest(
        text = json.str("text") ?: "",
        overdue = strings("overdue"),
        dueToday = strings("dueToday"),
        approvals = (json["approvals"] as? JsonArray)?.mapNotNull { (it as? JsonObject)?.str("title") } ?: emptyList(),
        events = (json["events"] as? JsonArray)?.mapNotNull { (it as? JsonObject)?.str("title") } ?: emptyList(),
        waiting = strings("waiting"),
    )
}

internal suspend fun AppStore.dailyDigest(): DailyDigest =
    digestOf(hostCall("dailyDigest", buildJsonObject {}, 60_000).jsonObject)

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
