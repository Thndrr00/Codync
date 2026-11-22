package com.codync.android

import com.codync.android.core.Entry
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import kotlinx.serialization.json.putJsonObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

private fun asked(id: String, status: String, thread: String? = null) = Entry(
    id = id, seq = 1, botId = "egan", rev = 1, kind = "notice", threadId = thread,
    data = buildJsonObject {
        put("text", "Asked Owen: $id")
        put("heading", "Asked Owen: $id")
        put("status", status)
        putJsonObject("botMessage") {
            put("sourceBotId", "egan"); put("targetBotId", "owen"); put("text", "Inspect $id")
        }
    },
)

class WaitingRepliesTest {
    @Test fun onlyPendingAskedRowsWaitAndAStoppedRowKeepsItsPlace() {
        val rows = WaitingReplies.rows(
            listOf(asked("waiting", "queued"), asked("done", "completed"), asked("stopped", "cancelled")),
            "egan", null, setOf("stopped"), { "Name:$it" },
        )
        assertEquals(listOf("stopped", "waiting"), rows.map { it.id })
        assertEquals(listOf(true, false), rows.map { it.stopped })
        assertEquals("Name:owen", rows.first().name)
        assertEquals("Inspect stopped", rows.first().request)
    }

    @Test fun aMessageOrAnotherThreadsAskDoesNotWait() {
        val message = asked("message", "queued").copy(data = buildJsonObject {
            put("text", "Messaged Owen: hi"); put("heading", "Messaged Owen: hi")
            putJsonObject("botMessage") {
                put("sourceBotId", "egan"); put("targetBotId", "owen"); put("text", "hi")
            }
        })
        val rows = WaitingReplies.rows(
            listOf(message, asked("done", "completed"), asked("thread", "queued", "other"), asked("mine", "queued", "root")),
            "egan", null, emptySet(), { it },
        )
        assertEquals(listOf("mine"), rows.map { it.id })
        assertTrue(rows.none { it.stopped })
        assertFalse(rows.isEmpty())
    }
}
