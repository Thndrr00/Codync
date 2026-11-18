package com.codync.android.core

import kotlinx.serialization.json.jsonObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class SubagentEntryTest {
    private fun decode(data: String): Entry =
        Entry.decode(WireJson.parseToJsonElement("""{"id":"s","seq":2,"botId":"bot","rev":3,"kind":"subagents","data":$data}""").jsonObject)

    @Test fun `a turn's subagents decode with their state, step and time`() {
        val entry = decode("""{"author":"bot","text":"Subagents: Read the tests, Fix them","subagents":[
            {"id":"t1","task":"Read the tests","state":"running","step":"Read src/lib.rs","startedAt":1000},
            {"id":"t2","task":"Fix them","state":"done","step":"Ran the tests","startedAt":1500,"endedAt":2500}]}""")
        assertTrue(entry.isChat)
        assertEquals(listOf("t1", "t2"), entry.subagents.map { it.id })
        assertEquals("Read src/lib.rs", entry.subagents[0].step)
        assertTrue(entry.subagents[0].isRunning)
        assertEquals(1000L, entry.subagents[0].startedAt)
        assertEquals("done", entry.subagents[1].shownState)
        assertEquals(2500L, entry.subagents[1].endedAt)
        assertFalse(entry.subagents[1].isRunning)
    }

    @Test fun `a subagent without a state is one still working`() {
        val entry = decode("""{"subagents":[{"id":"t1","task":"Read the tests"}]}""")
        assertEquals("running", entry.subagents.single().shownState)
        assertTrue(entry.subagents.single().isRunning)
    }

    @Test fun `a card this app cannot read never costs the conversation its message`() {
        val entry = decode("""{"text":"Subagents: Read the tests","subagents":{"unreadable":true}}""")
        assertTrue(entry.isChat)
        assertTrue(entry.subagents.isEmpty())
    }
}
