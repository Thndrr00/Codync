package com.codync.android

import com.codync.android.core.BotDraft
import org.junit.Assert.assertEquals
import org.junit.Test

class NewBotDraftKeepTest {
    @Test fun `leaving and reopening keeps the chosen model and effort`() {
        val saved = BotDraft(model = "gpt-5", modelOptions = mapOf("effort" to "high"))
        val kept = NewBotDraftKeep.keep(saved, BotDraft())
        assertEquals("gpt-5", kept.model)
        assertEquals(mapOf("effort" to "high"), kept.modelOptions)
    }

    @Test fun `a fresh choice wins and an absent draft changes nothing`() {
        val saved = BotDraft(model = "old-model", modelOptions = mapOf("effort" to "low"))
        val fresh = BotDraft(model = "new-model", modelOptions = mapOf("effort" to "high"))
        val kept = NewBotDraftKeep.keep(saved, fresh)
        assertEquals("new-model", kept.model)
        assertEquals(mapOf("effort" to "high"), kept.modelOptions)
        assertEquals(fresh, NewBotDraftKeep.keep(null, fresh))
    }
}
