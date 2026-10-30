package com.codync.android.core

import java.net.URI
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.decodeFromJsonElement

/** `createAppLink` returns the generated URL beside its link metadata. */
@Serializable data class AppLinkResult(val link: AppLink, val url: String) {
    /** Never substitute the app owner's URL for the newly granted capability. */
    fun shareUrl(): String {
        require(miniAppAccess(url) != MiniAppAccess.Unavailable) { "The computer did not return a usable app link." }
        val path = URI(url).path
        require(path == "/apps/${link.botId}/${link.slug}/${link.token}/") { "The app link does not match the generated permission." }
        return url
    }
}

enum class MiniAppAccess { Remote, ComputerOnly, Unavailable }

/** A host's loopback address belongs to the computer, never to this phone. */
fun miniAppAccess(url: String): MiniAppAccess {
    val uri = runCatching { URI(url) }.getOrNull() ?: return MiniAppAccess.Unavailable
    if (uri.scheme?.lowercase() !in listOf("http", "https") || uri.host.isNullOrBlank()) return MiniAppAccess.Unavailable
    val host = uri.host.lowercase().removePrefix("[").removeSuffix("]").trimEnd('.')
    return if (host == "localhost" || host.endsWith(".localhost") || host.startsWith("127.") || host == "::1")
        MiniAppAccess.ComputerOnly else MiniAppAccess.Remote
}

/** Published notices inherit their containing conversation's bot id when the payload omits it. */
fun botAppFromNotice(botId: String, payload: JsonObject): BotApp {
    val complete = if (payload["botId"] == null) JsonObject(payload + ("botId" to JsonPrimitive(botId))) else payload
    return WireJson.decodeFromJsonElement(complete)
}
