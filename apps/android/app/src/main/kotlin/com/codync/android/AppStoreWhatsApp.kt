package com.codync.android

import com.codync.android.core.WhatsApp
import com.codync.android.core.WhatsAppLink
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

/** The computer's linked WhatsApp number. Null from an older host. */
suspend fun AppStore.whatsapp(): WhatsAppLink? = runCatching { WhatsApp.decode(hostQuery("whatsappState", buildJsonObject { })) }.getOrNull()

/** Starts the link; the host emits the QR code as an event. */
suspend fun AppStore.whatsappConnect(): JsonElement = hostQuery("whatsappConnect", buildJsonObject { })

/** Links [phone] with a pairing code to type in WhatsApp instead of scanning a QR code. */
suspend fun AppStore.whatsappPair(phone: String): WhatsAppLink =
    WhatsApp.decode(hostQuery("whatsappPair", body("phone" to phone.trim())))

/** Forgets the number on this computer. */
suspend fun AppStore.whatsappLogout(): JsonElement = hostQuery("whatsappLogout", buildJsonObject { })

/** Sends the number's messages to [botId], or to none when null. */
suspend fun AppStore.whatsappSetBot(botId: String?): JsonElement = hostQuery("whatsappSetBot", body("botId" to botId))

/** Lets a chat or person write, or takes them out. Returns the list as it is then. */
suspend fun AppStore.whatsappSetAllowed(jid: String, allowed: Boolean): List<String> {
    val answer = hostQuery("whatsappSetAllowed", buildJsonObject { put("jid", jid); put("allowed", allowed) })
    return WhatsApp.decode(answer).allowed
}

/** Whether a request gets the "Working on it…" line before its answer; false sends only the answer. */
suspend fun AppStore.whatsappSetWorkingMessage(on: Boolean): Boolean =
    WhatsApp.decode(hostQuery("whatsappSetWorkingMessage", buildJsonObject { put("enabled", on) })).workingMessage
