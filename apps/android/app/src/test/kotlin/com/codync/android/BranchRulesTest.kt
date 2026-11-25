package com.codync.android

import com.codync.android.core.Entry
import com.codync.android.core.ThreadInfo
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class BranchRulesTest {
    private fun message(seq: Long, kind: String = "agent", final: Boolean = true) = Entry(
        id = "e$seq", seq = seq, botId = "bot", rev = seq, kind = kind,
        data = buildJsonObject { put("text", "message"); put("final", final) },
    )

    @Test fun `branch is offered only on the last final answer of an idle turn`() {
        val final = message(3)
        assertTrue(branchAllowed(final, final, "idle"))
        assertFalse(branchAllowed(final, message(4), "idle"), "an older answer is not the last message")
        assertFalse(branchAllowed(message(3, kind = "user"), message(3, kind = "user"), "idle"))
        assertFalse(branchAllowed(message(3, final = false), message(3, final = false), "idle"))
        assertFalse(branchAllowed(final, final, "working"), "a running turn cannot be branched")
    }

    @Test fun `each branch click gets its own request id`() {
        val first = branchRequestId()
        assertTrue(first.startsWith("branch-"))
        assertFalse(first == branchRequestId())
    }

    @Test fun `branches sharing a source message nest together and a lone branch stays flat`() {
        val first = ThreadInfo("a", fromEntry = "root", branchText = "Why?")
        val second = ThreadInfo("b", fromEntry = "root", branchText = "Why?")
        val own = ThreadInfo("c")
        val other = ThreadInfo("d", fromEntry = "other", branchText = "Else")
        val groups = nestThreads(listOf(first, second, own, other))
        assertEquals(listOf(listOf("a", "b"), listOf("c"), listOf("d")), groups.map { it.children.map { c -> c.id } })
        assertEquals("root", groups[0].source)
        assertEquals("Why?", groups[0].sourceText)
        assertEquals(null, groups[1].source)
    }
}
