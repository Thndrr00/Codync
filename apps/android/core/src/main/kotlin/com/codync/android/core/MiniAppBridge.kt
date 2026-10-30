package com.codync.android.core

import java.net.URI
import kotlinx.serialization.Serializable

/** The virtual origin a phone shows a mini-app under; every request to it goes through the host channel. */
const val MINI_APP_ORIGIN = "https://codync-app.invalid"

/** Who a mini-app address belongs to: the bot, the app and the link token that grants access. */
data class MiniAppAddress(val botId: String, val slug: String, val token: String) {
    val pageUrl: String get() = "$MINI_APP_ORIGIN/apps/$botId/$slug/$token/"

    /** Path inside the app for a request to [MINI_APP_ORIGIN], or null when it leaves this app. */
    fun relativePath(path: String): String? {
        val prefix = "/apps/$botId/$slug/$token/"
        return if (path.startsWith(prefix)) path.removePrefix(prefix) else null
    }

    companion object {
        private val shape = Regex("^/apps/([^/]+)/([^/]+)/([^/]+)/?$")

        /** Parses the owner or share URL the host returns, whatever address it carries. */
        fun parse(url: String): MiniAppAddress? {
            val path = runCatching { URI(url).path }.getOrNull() ?: return null
            val (bot, slug, token) = shape.matchEntire(path)?.destructured ?: return null
            return MiniAppAddress(bot, slug, token)
        }
    }
}

/** `appRequest` answer: [body] is base64. */
@Serializable data class MiniAppAnswer(
    val status: Int,
    val contentType: String = "",
    val csp: String = "",
    val body: String = "",
)

/** The host's sandbox directive would give the page an opaque origin; the virtual origin is already isolated. */
fun miniAppPolicy(csp: String): String =
    csp.split(';').map { it.trim() }.filter { it.isNotEmpty() && !it.startsWith("sandbox") }.joinToString("; ")
