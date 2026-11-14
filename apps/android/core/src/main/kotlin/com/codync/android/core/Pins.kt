package com.codync.android.core

import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonObject

/** Who pinned a message: a person (their device, or none at the computer itself) or the chat's agent. */
@Serializable
data class Pinner(val kind: String = "person", val device: String? = null, val botId: String? = null)

/** A pinned message of a chat or one of its threads. */
@Serializable
data class PinnedMessage(
    val messageId: String,
    val threadId: String? = null,
    val threadTitle: String? = null,
    val pinnedAt: Long = 0,
    val pinnedBy: Pinner = Pinner(),
    /** Who wrote it: a person, or this chat's agent. */
    val author: String = "user",
    val authorBotId: String? = null,
    val createdAt: Long = 0,
    /** The start of the message, or the names of its files when it has no text. */
    val text: String = "",
)

/** Who pinned it, or null when it isn't pinned. */
val Entry.pinned: Pinner?
    get() = (data["pinned"] as? JsonObject)?.let { pin ->
        (pin["by"] as? JsonObject)?.let { by -> runCatching { WireJson.decodeFromJsonElement(Pinner.serializer(), by) }.getOrNull() }
    }
