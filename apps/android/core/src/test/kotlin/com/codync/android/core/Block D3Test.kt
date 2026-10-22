package com.codync.android.core

import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class NewMessageCounterTest {
    @Test fun `counts distinct new ids after leaving the bottom`() {
        val counter = NewMessageCounter()
        assertEquals(0, counter.update(listOf("a", "b"), atBottom = false))
        assertEquals(1, counter.update(listOf("a", "b", "c"), atBottom = false))
        assertEquals(2, counter.update(listOf("a", "b", "c", "d"), atBottom = false))
    }

    @Test fun `a streaming edit of the same entry is not a new message`() {
        val counter = NewMessageCounter()
        counter.update(listOf("a"), atBottom = false)
        counter.update(listOf("a", "b"), atBottom = false)
        assertEquals(1, counter.update(listOf("a", "b"), atBottom = false))
    }

    @Test fun `loading older history does not count`() {
        val counter = NewMessageCounter()
        counter.update(listOf("c", "d"), atBottom = false)
        assertEquals(0, counter.update(listOf("a", "b", "c", "d"), atBottom = false))
        assertEquals(1, counter.update(listOf("a", "b", "c", "d", "e"), atBottom = false))
    }

    @Test fun `the readers own messages do not count`() {
        val counter = NewMessageCounter()
        counter.update(listOf("a"), atBottom = false)
        assertEquals(1, counter.update(listOf("a", "user-1", "b"), atBottom = false, own = setOf("user-1")))
    }

    @Test fun `returning to the bottom resets and the next departure starts again`() {
        val counter = NewMessageCounter()
        counter.update(listOf("a"), atBottom = false)
        counter.update(listOf("a", "b"), atBottom = false)
        assertEquals(0, counter.update(listOf("a", "b"), atBottom = true))
        assertEquals(0, counter.count)
        assertEquals(0, counter.update(listOf("a", "b"), atBottom = false))
        assertEquals(1, counter.update(listOf("a", "b", "c"), atBottom = false))
    }

    @Test fun `separate scopes keep separate counts`() {
        val chat = NewMessageCounter()
        val thread = NewMessageCounter()
        chat.update(listOf("a"), atBottom = false)
        thread.update(listOf("r"), atBottom = false)
        chat.update(listOf("a", "b"), atBottom = false)
        assertEquals(1, chat.count)
        assertEquals(0, thread.count)
    }
}

class TraceFilterTest {
    private fun entry(kind: String, vararg data: Pair<String, String>) =
        Entry("$kind-${data.size}", 1L, "bot", rev = 1L, kind = kind, data = JsonObject(data.associate { it.first to JsonPrimitive(it.second) }))

    @Test fun `filters follow the event type and never the words in the text`() {
        val chatty = entry("agent", "text" to "the tool failed and I was thinking about a plan")
        assertTrue(TraceFilter.Messages.matches(chatty))
        assertFalse(TraceFilter.Tools.matches(chatty))
        assertFalse(TraceFilter.Thinking.matches(chatty))
        assertFalse(TraceFilter.Problems.matches(chatty))
        assertTrue(TraceFilter.Thinking.matches(entry("thought")))
        assertTrue(TraceFilter.Tools.matches(entry("tool")))
        assertTrue(TraceFilter.Plans.matches(entry("plan")))
        assertTrue(TraceFilter.Approvals.matches(entry("permission")))
    }

    @Test fun `problems are failed tools and error notices`() {
        assertTrue(TraceFilter.Problems.matches(entry("tool", "status" to "failed")))
        assertFalse(TraceFilter.Problems.matches(entry("tool", "status" to "completed")))
        assertTrue(TraceFilter.Problems.matches(entry("notice", "style" to "error")))
        assertFalse(TraceFilter.Problems.matches(entry("notice", "style" to "divider")))
    }

    @Test fun `all keeps every entry and counts add up`() {
        val list = listOf(entry("user"), entry("tool", "status" to "failed"), entry("notice"))
        val counts = TraceFilter.counts(list)
        assertEquals(3, counts[TraceFilter.All])
        assertEquals(1, counts[TraceFilter.Tools])
        assertEquals(1, counts[TraceFilter.Problems])
    }

    @Test fun `the scroll anchor stays on its turn or moves to the next shown one`() {
        assertEquals(1, TraceFilter.anchorIndex(listOf(1L, 2L, 3L), 2L, listOf(1L, 2L)))
        assertEquals(1, TraceFilter.anchorIndex(listOf(1L, 2L, 3L), 2L, listOf(1L, 3L)))
        assertEquals(1, TraceFilter.anchorIndex(listOf(1L, 2L, 3L), 3L, listOf(1L, 2L)))
        assertEquals(2, TraceFilter.anchorIndex(listOf(1L, 2L, 3L), null, listOf(1L, 2L, 3L)))
        assertNull(TraceFilter.anchorIndex(listOf(1L, 2L), 1L, emptyList()))
    }
}
