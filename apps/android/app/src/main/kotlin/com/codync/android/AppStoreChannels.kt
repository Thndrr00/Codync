package com.codync.android

import com.codync.android.core.Channels
import com.codync.android.core.DiscordBinding
import com.codync.android.core.DiscordStatus
import com.codync.android.core.MatrixBinding
import com.codync.android.core.MatrixStatus
import com.codync.android.core.SignalBinding
import com.codync.android.core.SignalStatus
import com.codync.android.core.SlackStatus
import com.codync.android.core.TeamsStatus
import com.codync.android.core.TelegramBot
import com.codync.android.core.WireJson
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonArray
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

/** The messaging channels of the computer: its Telegram bots and Slack connection. Null from an older host. */
suspend fun AppStore.telegramBots(): List<TelegramBot>? = runCatching { Channels.decodeTelegram(hostQuery("telegramStatus", buildJsonObject { })) }.getOrNull()

suspend fun AppStore.slackStatus(): SlackStatus? = runCatching { Channels.decodeSlack(hostQuery("slackStatus", buildJsonObject { })) }.getOrNull()

suspend fun AppStore.disconnectSlack(): SlackStatus = Channels.decodeSlack(hostQuery("disconnectSlack", buildJsonObject { }))

suspend fun AppStore.discordStatus(): DiscordStatus? = runCatching { Channels.decodeDiscord(hostQuery("discordStatus", buildJsonObject { })) }.getOrNull()

suspend fun AppStore.disconnectDiscord(): DiscordStatus = Channels.decodeDiscord(hostQuery("disconnectDiscord", buildJsonObject { }))

/** Sets the Discord bot and its bound channels up; the host checks the token before saving anything. */
suspend fun AppStore.setDiscord(
    guildId: String,
    botUserId: String,
    botToken: String,
    bindings: List<DiscordBinding>,
): DiscordStatus = Channels.decodeDiscord(hostQuery("setDiscordChannel", buildJsonObject {
    put("guildId", guildId.trim())
    put("botUserId", botUserId.trim())
    put("botToken", botToken.trim())
    put("bindings", buildJsonArray {
        for (binding in bindings) add(buildJsonObject {
            put("channelId", binding.channelId.trim())
            put("botId", binding.botId)
            put("allowedUsers", buildJsonArray { for (user in binding.allowedUsers) add(JsonPrimitive(user.trim())) })
        })
    })
}, timeoutMillis = 60_000))

/** Sets the Matrix session and its bound rooms up; the host signs in and checks every room first. */
suspend fun AppStore.setMatrix(
    homeserver: String,
    user: String,
    accessToken: String,
    password: String,
    bindings: List<MatrixBinding>,
): MatrixStatus = Channels.decodeMatrix(hostQuery("setMatrixChannel", buildJsonObject {
    put("homeserver", homeserver.trim())
    put("user", user.trim())
    put("accessToken", accessToken.trim())
    put("password", password)
    put("bindings", buildJsonArray {
        for (binding in bindings) add(buildJsonObject {
            put("roomId", binding.roomId.trim())
            put("botId", binding.botId)
            put("allowedUsers", buildJsonArray { for (person in binding.allowedUsers) add(JsonPrimitive(person.trim())) })
        })
    })
}, timeoutMillis = 60_000))

suspend fun AppStore.matrixStatus(): MatrixStatus? = runCatching { Channels.decodeMatrix(hostQuery("matrixStatus", buildJsonObject { })) }.getOrNull()

suspend fun AppStore.disconnectMatrix(): MatrixStatus = Channels.decodeMatrix(hostQuery("disconnectMatrix", buildJsonObject { }))

suspend fun AppStore.signalStatus(): SignalStatus? = runCatching {
    WireJson.decodeFromJsonElement(SignalStatus.serializer(), hostQuery("signalStatus", buildJsonObject { }))
}.getOrNull()

suspend fun AppStore.disconnectSignal(): SignalStatus =
    WireJson.decodeFromJsonElement(SignalStatus.serializer(), hostQuery("disconnectSignal", buildJsonObject { }))

/** Sets the signal-cli daemon and its chats up; the host checks the daemon and its linked number first. */
suspend fun AppStore.setSignal(number: String, daemon: String, bindings: List<SignalBinding>): SignalStatus =
    WireJson.decodeFromJsonElement(SignalStatus.serializer(), hostQuery("setSignalChannel", buildJsonObject {
        put("number", number.trim())
        put("daemon", daemon.trim())
        put("bindings", buildJsonArray {
            for (binding in bindings) add(buildJsonObject {
                put("chat", binding.chat.trim())
                put("botId", binding.botId)
                put("allowedUsers", buildJsonArray { for (person in binding.allowedUsers) add(JsonPrimitive(person.trim())) })
            })
        })
    }, timeoutMillis = 60_000))

suspend fun AppStore.teamsStatus(): TeamsStatus? = runCatching {
    WireJson.decodeFromJsonElement(TeamsStatus.serializer(), hostQuery("teamsStatus", buildJsonObject { }))
}.getOrNull()

suspend fun AppStore.disconnectTeams(): TeamsStatus =
    WireJson.decodeFromJsonElement(TeamsStatus.serializer(), hostQuery("disconnectTeams", buildJsonObject { }))

/** Sets the Azure Bot registration up; the host gets a Bot Framework token before saving it. */
suspend fun AppStore.setTeams(
    appId: String,
    tenantId: String,
    appPassword: String,
    managedIdentity: Boolean,
    botId: String,
    allowedUsers: List<String>,
    allowedTenants: List<String>,
    memory: String,
    icon: String,
): TeamsStatus = WireJson.decodeFromJsonElement(TeamsStatus.serializer(), hostQuery("setTeamsChannel", buildJsonObject {
    put("appId", appId.trim())
    put("tenantId", tenantId.trim())
    put("appPassword", appPassword)
    put("managedIdentity", managedIdentity)
    put("botId", botId)
    put("allowedUsers", buildJsonArray { for (user in allowedUsers) add(JsonPrimitive(user.trim())) })
    put("allowedTenants", buildJsonArray { for (tenant in allowedTenants) add(JsonPrimitive(tenant.trim())) })
    put("memory", memory)
    put("icon", icon)
}, timeoutMillis = 60_000))

/** Adds a Telegram bot the host will talk through, answered by [botId]; its token stays on the computer. */suspend fun AppStore.addTelegramBot(name: String, token: String, botId: String): List<TelegramBot> =
    Channels.decodeTelegram(hostQuery("addTelegramBot",
        body("name" to name.trim(), "token" to token.trim(), "botId" to botId), timeoutMillis = 60_000))

suspend fun AppStore.removeTelegramBot(name: String): List<TelegramBot> =
    Channels.decodeTelegram(hostQuery("removeTelegramBot", body("name" to name)))

/** Makes a one-time pairing code for the Telegram bot [name]; the answer also lists the bots. */
suspend fun AppStore.pairTelegram(name: String): JsonElement = hostQuery("pairTelegram", body("name" to name))

/** Lets a Telegram user write to the bot [name], or takes them out. Returns the bots as they are then. */
suspend fun AppStore.setTelegramAllowed(name: String, userId: Long, allowed: Boolean): List<TelegramBot> =
    Channels.decodeTelegram(hostQuery("setTelegramAllowed", buildJsonObject {
        put("name", name)
        put("userId", userId)
        put("allowed", allowed)
    }))

/** Saves what the bot should know about a channel (`provider`: telegram, slack or discord). */
suspend fun AppStore.setChannelMemory(provider: String, channel: String, memory: String): JsonElement =
    hostQuery("setChannelMemory", buildJsonObject {
        put("provider", provider)
        put("channel", channel)
        put("memory", memory)
    })


/** Small facts a channel keeps, given to the bot with its memory. */
suspend fun AppStore.channelNotes(provider: String, channel: String): List<String> {
    val answer = hostQuery("channelNotes", buildJsonObject { put("provider", provider); put("channel", channel) }).jsonObject
    return answer["notes"]?.jsonArray?.map { it.jsonPrimitive.content }.orEmpty()
}

suspend fun AppStore.addChannelNote(provider: String, channel: String, text: String): List<String> {
    val answer = hostQuery("addChannelNote", buildJsonObject {
        put("provider", provider); put("channel", channel); put("text", text)
    }).jsonObject
    return answer["notes"]?.jsonArray?.map { it.jsonPrimitive.content }.orEmpty()
}

suspend fun AppStore.removeChannelNote(provider: String, channel: String, index: Int): List<String> {
    val answer = hostQuery("removeChannelNote", buildJsonObject {
        put("provider", provider); put("channel", channel); put("index", index)
    }).jsonObject
    return answer["notes"]?.jsonArray?.map { it.jsonPrimitive.content }.orEmpty()
}

/** Gives a channel a picture (an SF Symbol name), or clears it. */
suspend fun AppStore.setChannelIcon(provider: String, channel: String, icon: String): JsonElement =
    hostQuery("setChannelIcon", buildJsonObject {
        put("provider", provider)
        put("channel", channel)
        put("icon", icon)
    })

/** The conversations of [botId] linked to a Codync thread (card K13). */
suspend fun AppStore.conversationLinks(botId: String): List<ConversationLink> =
    ConversationLink.decodeList(hostQuery("conversationLinks", body("botId" to botId)))

/** Links a channel conversation to [threadId]; every link of the bot comes back. */
suspend fun AppStore.linkConversation(botId: String, provider: String, channel: String, thread: String,
    threadId: String): List<ConversationLink> =
    ConversationLink.decodeList(hostQuery("linkConversation", buildJsonObject {
        put("botId", botId)
        put("provider", provider)
        put("channel", channel)
        put("thread", thread)
        put("threadId", threadId)
    }))

/** Takes a channel conversation's link away; its next message starts a conversation of its own. */
suspend fun AppStore.unlinkConversation(botId: String, provider: String, channel: String,
    thread: String): List<ConversationLink> =
    ConversationLink.decodeList(hostQuery("unlinkConversation", buildJsonObject {
        put("botId", botId)
        put("provider", provider)
        put("channel", channel)
        put("thread", thread)
    }))
