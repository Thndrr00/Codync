package com.codync.android.core

import kotlinx.serialization.SerializationException
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.json.jsonObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Test

class MiniAppAccessTest {
    @Test fun `share response keeps the generated capability URL outside link metadata`() {
        val result = WireJson.decodeFromString<AppLinkResult>("""
            {"link":{"token":"viewer-token","botId":"bot","slug":"tracker","mode":"view"},
             "url":"http://127.0.0.1:8787/apps/bot/tracker/viewer-token/"}
        """.trimIndent())
        assertEquals("view", result.link.mode)
        assertEquals("http://127.0.0.1:8787/apps/bot/tracker/viewer-token/", result.shareUrl())
        assertFalse(result.shareUrl().contains("owner-token"))
    }

    @Test(expected = IllegalArgumentException::class)
    fun `an owner URL cannot substitute for the generated view link`() {
        AppLinkResult(AppLink("viewer-token", "bot", "tracker", mode = "view"),
            "http://127.0.0.1:8787/apps/bot/tracker/owner-token/").shareUrl()
    }

    @Test(expected = SerializationException::class)
    fun `a missing generated URL is an error rather than a relative fallback`() {
        WireJson.decodeFromString<AppLinkResult>("""{"link":{"token":"view","botId":"bot","slug":"app"}}""")
    }

    @Test fun `host loopback URLs are unavailable on the phone without rewriting their host`() {
        for (url in listOf("http://127.0.0.1:8787/apps/", "http://localhost:8787/", "http://[::1]:8787/")) {
            assertEquals(MiniAppAccess.ComputerOnly, miniAppAccess(url))
        }
        assertEquals(MiniAppAccess.Unavailable, miniAppAccess("/apps/bot/app/token/"))
        assertEquals(MiniAppAccess.Unavailable, miniAppAccess("javascript:alert(1)"))
        assertEquals(MiniAppAccess.Remote, miniAppAccess("https://apps.example.com/apps/bot/app/token/"))
    }

    @Test fun `published app notice uses its containing bot when host omits botId`() {
        val payload = WireJson.parseToJsonElement("""
            {"slug":"tracker","title":"Tracker","entry":"index.html","version":2,
             "ownerToken":"owner","url":"http://127.0.0.1:8787/apps/bot/tracker/owner/"}
        """.trimIndent()).jsonObject
        val app = botAppFromNotice("bot", payload)
        assertEquals("bot", app.botId)
        assertEquals("Tracker", app.title)
        assertEquals(2, app.version)
    }

    @Test fun `lock summary hides identifying details even for a stale feed with errors`() {
        val feed = WidgetFeed(computerId = "private-id", computerName = "Private computer", botsAt = 1,
            bots = listOf(WidgetBot("a", "Private reviewer", "needsInput"), WidgetBot("b", "Private builder", "working"),
                WidgetBot("c", "Private failed bot", "error")))
        val summary = lockScreenSummary(feed, nowMillis = DEFAULT_LOCK_WIDGET_STALE_MS + 2)
        assertEquals(WidgetFreshness.Stale, summary.freshness)
        assertEquals(1, summary.workingCount)
        assertEquals(2, summary.attentionCount)
        assertNull(summary.computerName)
        assertNull(summary.leadBotName)
    }
}

class MiniAppBridgeTest {
    @Test fun `address comes from the owner URL whatever host it names`() {
        val address = MiniAppAddress.parse("http://127.0.0.1:19222/apps/bot/tracker/owner/")
        assertEquals(MiniAppAddress("bot", "tracker", "owner"), address)
        assertEquals("https://codync-app.invalid/apps/bot/tracker/owner/", address?.pageUrl)
        assertNull(MiniAppAddress.parse("http://127.0.0.1/health"))
        assertNull(MiniAppAddress.parse("not a url"))
    }

    @Test fun `requests leaving the app are rejected`() {
        val address = MiniAppAddress("bot", "tracker", "owner")
        assertEquals("_data/notes", address.relativePath("/apps/bot/tracker/owner/_data/notes"))
        assertEquals("", address.relativePath("/apps/bot/tracker/owner/"))
        assertNull(address.relativePath("/apps/bot/other/owner/"))
        assertNull(address.relativePath("/apps/bot/tracker/other-token/"))
    }

    @Test fun `policy drops the sandbox directive only`() {
        assertEquals("default-src 'self'; img-src *", miniAppPolicy("sandbox allow-scripts; default-src 'self'; img-src *;"))
    }
}
