package com.codync.android

import com.codync.android.core.BotDraft
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import org.junit.Assert.assertEquals
import org.junit.Test

class NewBotAgentDefaultsTest {
    private fun hello(claude: Boolean, codex: Boolean) = AppState(hello = Json.parseToJsonElement("""
        {"backends":[{"id":"claude","available":$claude,"curated":true},
                     {"id":"codex","available":$codex,"curated":true}]}
    """.trimIndent()).jsonObject)

    @Test fun `explicit unavailable agent and model survive later hello refreshes`() {
        val defaults = NewBotAgentDefaults()
        val initial = defaults.refreshed(BotDraft(), hello(claude = false, codex = true))
        assertEquals("codex", initial.backend)
        val chosen = defaults.edited(initial, initial.copy(backend = "claude", model = "chosen-model",
            modelOptions = mapOf("effort" to "high")))
        assertEquals(chosen, defaults.refreshed(chosen, hello(claude = false, codex = true)))
        assertEquals(chosen, defaults.refreshed(chosen, hello(claude = true, codex = false)))
    }

    @Test fun `an untouched default can follow newly available agents`() {
        val defaults = NewBotAgentDefaults()
        val waiting = defaults.refreshed(BotDraft(), AppState())
        assertEquals("claude", waiting.backend)
        val codex = defaults.refreshed(waiting, hello(claude = false, codex = true))
        assertEquals("codex", codex.backend)
        assertEquals("claude", defaults.refreshed(codex, hello(claude = true, codex = false)).backend)
    }

    @Test fun `a model choice also preserves its agent when availability changes`() {
        val defaults = NewBotAgentDefaults()
        val initial = defaults.refreshed(BotDraft(), hello(claude = true, codex = false))
        val chosen = defaults.edited(initial, initial.copy(model = "specific-model"))
        assertEquals(chosen, defaults.refreshed(chosen, hello(claude = false, codex = true)))
    }
}
