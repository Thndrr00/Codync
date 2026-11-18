package com.codync.android

import com.codync.android.core.Entry
import com.codync.android.core.EntrySubagent
import com.codync.android.core.WireJson
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.encodeToJsonElement
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class SubagentCardTest {
    private fun subagent(id: String, state: String, startedAt: Long, endedAt: Long? = null) =
        EntrySubagent(id = id, task = id, state = state, startedAt = startedAt, endedAt = endedAt)

    private fun card(id: String, vararg subagents: EntrySubagent) = Entry(id, 1, "bot", rev = 1, kind = "subagents",
        data = JsonObject(mapOf("subagents" to WireJson.encodeToJsonElement<List<EntrySubagent>>(subagents.toList()))))

    @Test fun `a time reads in seconds, then minutes and seconds`() {
        assertEquals("0s", subagentDuration(0))
        assertEquals("12s", subagentDuration(12_400))
        assertEquals("1m 05s", subagentDuration(65_000))
    }

    @Test fun `all of a turn's subagents take from the first start to now or the last end`() {
        val running = listOf(subagent("t1", "done", 1_000, endedAt = 3_000), subagent("t2", "running", 2_000))
        assertEquals("12s", subagentDuration(running, now = 13_000))
        val ended = listOf(subagent("t1", "failed", 1_000, endedAt = 3_000), subagent("t2", "stopped", 2_000, endedAt = 4_000))
        assertEquals("3s", subagentDuration(ended, now = 90_000))
    }

    @Test fun `the dock follows the newest card only while a subagent works`() {
        val running = card("new", subagent("t1", "running", 1_000))
        val ended = card("old", subagent("t1", "done", 1_000, endedAt = 2_000))
        assertEquals("new", listOf(ended, running).activeSubagentCard()?.id)
        assertNull(listOf(ended, card("also ended", subagent("t2", "stopped", 1_000))).activeSubagentCard())
    }
}
