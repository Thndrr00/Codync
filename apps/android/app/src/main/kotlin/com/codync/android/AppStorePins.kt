package com.codync.android

import com.codync.android.core.*
import kotlinx.serialization.json.*

/** Pins or unpins a message; the host answers with the message's new rev, which the mirror takes. */
fun AppStore.setPinned(entry: Entry) = command(
    "pinMessage",
    buildJsonObject { put("botId", entry.botId); put("messageId", entry.id); put("pinned", entry.pinned == null) },
    "pin:" + entry.id,
)

/** The bot's pins, newest first, across its chats (`pinnedMessages`). */
internal suspend fun AppStore.pinnedMessages(botId: String): List<PinnedMessage> =
    WireJson.decodeFromJsonElement(hostQuery("pinnedMessages", body("botId" to botId)).jsonObject.getValue("pins"))
