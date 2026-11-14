package com.codync.android.core

import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.jsonObject

/** A Telegram bot of the computer's host, as `telegramStatus` lists it. Never carries the token. */
@Serializable data class TelegramBot(
    val name: String,
    val username: String = "",
    val botId: String = "",
    val botName: String = "",
    val allowed: List<Long> = emptyList(),
    val memory: String = "",
    val icon: String = "",
    /** `ok`, `starting`, or `error: …`. */
    val state: String = "starting",
    val lastReceivedAt: Long? = null,
    val pairingExpiresAt: Long? = null,
)

/** The Slack connection, as `slackStatus` reports it. */
@Serializable data class SlackBinding(val channelId: String, val botId: String = "", val allowedUsers: List<String> = emptyList(), val memory: String = "", val icon: String = "")

@Serializable data class SlackStatus(val configured: Boolean = false, val socketMode: Boolean = false, val workspaceId: String? = null, val bindings: List<SlackBinding>? = null)

/** One bound Discord channel: the host bot that answers it and the people allowed in it. */
@Serializable data class DiscordBinding(val channelId: String, val botId: String = "", val allowedUsers: List<String> = emptyList(), val memory: String = "", val icon: String = "")

/** The Discord connection, as `discordStatus` reports it. */
@Serializable data class DiscordStatus(
    val configured: Boolean = false,
    val guildId: String? = null,
    val botUserId: String? = null,
    /** Null while Discord isn't connected at all; the host sends no list then. */
    val bindings: List<DiscordBinding>? = null,
    /** `ok`, `starting`, or `error: …`. */
    val state: String = "starting",
)

/** One bound Matrix room: the host bot that answers it and the people allowed in it. */
@Serializable data class MatrixBinding(
    val roomId: String,
    val botId: String = "",
    val allowedUsers: List<String> = emptyList(),
    val memory: String = "",
    val icon: String = "",
)

/** The Matrix connection, as `matrixStatus` reports it. */
@Serializable data class MatrixStatus(
    val configured: Boolean = false,
    val homeserver: String? = null,
    val userId: String? = null,
    val deviceId: String? = null,
    val bindings: List<MatrixBinding>? = null,
    /** `ok`, `starting`, or `error: …`. */
    val state: String = "starting",
)

/** One Signal chat bound to a bot: a phone number or `group:<id>`. */
@Serializable data class SignalBinding(
    val chat: String,
    val botId: String = "",
    val allowedUsers: List<String> = emptyList(),
    val memory: String = "",
    val icon: String = "",
)

/** The signal-cli connection, as `signalStatus` reports it. */
@Serializable data class SignalStatus(
    val configured: Boolean = false,
    val number: String? = null,
    val daemon: String? = null,
    val bindings: List<SignalBinding>? = null,
    /** `ok`, `starting`, `notRunning: …` or `error: …`. */
    val state: String = "starting",
    /** The exact signal-cli commands when the daemon is not set up. */
    val setup: String? = null,
)

/** The Microsoft Teams connection, as `teamsStatus` reports it. */
@Serializable data class TeamsStatus(
    val configured: Boolean = false,
    val appId: String? = null,
    val tenantId: String? = null,
    val botId: String? = null,
    val managedIdentity: Boolean = false,
    val allowedUsers: List<String> = emptyList(),
    val allowedTenants: List<String> = emptyList(),
    val memory: String = "",
    val icon: String = "",
    val endpoint: String? = null,
    val registration: String = "",
    val state: String = "starting",
)

/**
 * One messaging conversation linked to a Codync thread (card K13): what `conversationLinks` lists and
 * `linkConversation` and `unlinkConversation` change, so the app, a call and the channel share one conversation.
 */
@Serializable data class ConversationLink(
    val botId: String,
    /** `telegram`, `slack`, `discord` or `matrix`. */
    val source: String,
    val channel: String,
    /** The provider's thread inside the channel: a Slack thread's timestamp, empty for a Telegram chat or Discord channel. */
    val thread: String = "",
    /** The Codync thread the conversation writes into. */
    val rootId: String,
    val createdAt: Long = 0,
    val updatedAt: Long = 0,
) {
    val key: String get() = "$channel/$thread"

    companion object {
        fun decodeList(response: JsonElement): List<ConversationLink> =
            (response.jsonObject["links"] as? kotlinx.serialization.json.JsonArray).orEmpty()
                .mapNotNull { runCatching { WireJson.decodeFromJsonElement<ConversationLink>(it) }.getOrNull() }
    }
}

/** What the Channels screen shows, and the words it uses; the host owns the meaning. */
object Channels {
    /** A short name the host accepts for a Telegram bot. */
    fun nameOk(name: String): Boolean = name.length in 1..32 && name.all { it in 'a'..'z' || it in '0'..'9' || it == '-' }

    /** `123456:ABC…`, the shape BotFather gives. */
    fun tokenOk(token: String): Boolean = token.length <= 128 && Regex("^\\d+:[A-Za-z0-9_-]{30,}$").matches(token)

    /** A Discord snowflake. */
    fun idOk(id: String): Boolean = id.length in 1..32 && id.all { it in '0'..'9' }

    /** The three dot-separated parts of a bot token. */
    fun discordTokenOk(token: String): Boolean =
        token.length <= 200 && Regex("^[A-Za-z0-9_-]{20,}\\.[A-Za-z0-9_-]+\\.[A-Za-z0-9_-]{20,}$").matches(token)

    /** A homeserver address: https, or http to a server on the computer's own network. */
    fun matrixHomeserverOk(url: String): Boolean =
        url.trim().length <= 300 && Regex("^https?://[^\\s/]+(/[^\\s]*)?$").matches(url.trim())

    /** A full Matrix user id: `@name:server`. */
    fun matrixUserOk(id: String): Boolean = id.trim().length <= 255 && Regex("^@[^:\\s]+:[^\\s]+$").matches(id.trim())

    /** A room id or alias: `!opaque:server` or `#alias:server`. */
    fun matrixRoomOk(id: String): Boolean = id.trim().length <= 255 && Regex("^[!#][^:\\s]+:[^\\s]+$").matches(id.trim())

    fun matrixStateText(state: String): String = when (state) {
        "ok" -> "Connected"
        "starting" -> "Connecting…"
        else -> "Cannot reach Matrix"
    }

    /** A Signal chat: an E.164 number or `group:<id>`. */
    fun signalChatOk(value: String): Boolean {
        val chat = value.trim()
        return chat.length <= 512 && (Regex("^\\+[1-9]\\d{6,14}$").matches(chat) ||
            Regex("^group:[A-Za-z0-9+/=_-]{8,512}$").matches(chat))
    }

    fun signalNumberOk(value: String): Boolean = Regex("^\\+[1-9]\\d{6,14}$").matches(value.trim())

    fun signalPersonOk(value: String): Boolean =
        signalNumberOk(value) || Regex("^[0-9a-fA-F-]{32,64}$").matches(value.trim())

    fun signalStateText(state: String): String = when {
        state == "ok" -> "Connected"
        state == "starting" -> "Connecting…"
        state.startsWith("notRunning") -> "signal-cli is not running"
        else -> "Cannot reach signal-cli"
    }

    fun teamsStateText(state: String): String = when (state) {
        "ok" -> "Connected"
        "starting" -> "Connecting…"
        else -> "Not set up"
    }

    fun decodeTelegram(response: JsonElement): List<TelegramBot> =
        WireJson.decodeFromJsonElement(kotlinx.serialization.builtins.ListSerializer(TelegramBot.serializer()), response.jsonObject.getValue("bots"))

    fun decodeSlack(response: JsonElement): SlackStatus = WireJson.decodeFromJsonElement(SlackStatus.serializer(), response)

    fun decodeDiscord(response: JsonElement): DiscordStatus = WireJson.decodeFromJsonElement(DiscordStatus.serializer(), response)

    fun decodeMatrix(response: JsonElement): MatrixStatus = WireJson.decodeFromJsonElement(MatrixStatus.serializer(), response)

    fun discordStateText(state: String): String = when (state) {
        "ok" -> "Connected"
        "starting" -> "Connecting…"
        else -> "Cannot reach Discord"
    }

    /** The pairing code a `pairTelegram` answer carries. */
    fun pairingCode(response: JsonElement): String? =
        (response.jsonObject["result"] as? kotlinx.serialization.json.JsonObject)?.get("code")?.let { (it as? kotlinx.serialization.json.JsonPrimitive)?.content }

    fun stateText(state: String): String = when (state) {
        "ok" -> "Connected"
        "starting" -> "Starting…"
        else -> "Cannot reach Telegram"
    }

    fun lastReceivedText(at: Long?, now: Long): String {
        if (at == null) return "Nothing received yet"
        val minutes = maxOf(0L, (now - at) / 60_000)
        return when {
            minutes < 1 -> "Last message just now"
            minutes < 60 -> "Last message $minutes min ago"
            minutes < 60 * 24 -> "Last message ${minutes / 60} h ago"
            else -> "Last message ${minutes / (60 * 24)} d ago"
        }
    }

    fun pairingInstruction(code: String, username: String): String {
        val where = if (username.isEmpty()) "your Telegram bot" else "@$username"
        return "Send /start $code to $where in Telegram. The code works once, for 10 minutes."
    }

    fun allowedText(count: Int): String = if (count == 1) "1 person allowed" else "$count people allowed"
}
