package com.codync.android.core

import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.jsonObject

/** The computer's linked WhatsApp number, as `whatsappState` reports it. */
@Serializable data class WhatsAppLink(
    /** `disconnected`, `starting`, `qr`, `pairing`, `connected`, `loggedOut` or `error`. */
    val state: String = "disconnected",
    val qr: String? = null,
    /** The code to type in WhatsApp → Linked devices instead of scanning a QR code. */
    val pairCode: String? = null,
    val me: String? = null,
    val name: String? = null,
    val error: String? = null,
    /** The bot whose chat receives this number's messages. */
    val bot: String? = null,
    /** Chats and people allowed to write; empty means anyone. */
    val allowed: List<String> = emptyList(),
    /** People waiting for the owner's answer on a restricted number. */
    val requests: List<WhatsAppRequest> = emptyList(),
    /** People this number knows, with the facts they gave. */
    val people: List<WhatsAppPerson> = emptyList(),
)

/** Someone waiting for the owner to let them write to a linked number. */
@Serializable data class WhatsAppRequest(
    val code: String,
    val sender: String,
    val name: String? = null,
    /** When the request arrived, in Unix milliseconds. */
    val at: Long = 0,
)

/** A person a linked number knows: their name and the facts they gave. */
@Serializable data class WhatsAppPerson(
    val sender: String,
    val name: String? = null,
    val facts: List<String> = emptyList(),
)

/** What the WhatsApp screen shows, and the words it uses; the host owns the meaning. */
object WhatsApp {
    fun decode(response: JsonElement): WhatsAppLink = WireJson.decodeFromJsonElement(WhatsAppLink.serializer(), response.jsonObject)

    fun stateText(state: String): String = when (state) {
        "connected" -> "Linked"
        "starting" -> "Starting…"
        "qr" -> "Scan the code"
        "pairing" -> "Enter the code on your phone."
        "loggedOut" -> "Not linked"
        "error" -> "The link stopped"
        else -> "Not linked"
    }

    /** The code in two groups of four, as WhatsApp shows its field for it. */
    fun groupedCode(code: String): String = if (code.length == 8) "${code.take(4)}-${code.drop(4)}" else code
}
