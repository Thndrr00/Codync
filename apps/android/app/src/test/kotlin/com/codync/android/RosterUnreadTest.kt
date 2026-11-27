package com.codync.android

import com.codync.android.core.Entry
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/** The roster's "Mark unread" decision, pure so it can be checked without a host. */
class RosterUnreadTest {

    @Test
    fun `there is no unread action without a message`() {
        assertNull(markUnreadAction(null))
        assertEquals(UnreadAction("e3"), markUnreadAction("e3"))
    }

    @Test
    fun `the newest final agent reply is what the roster marks unread`() {
        val entries = listOf(
            chatEntry("u1", "user"),
            chatEntry("a1", "agent"),
            chatEntry("t1", "thought"),
            chatEntry("a2", "agent"),
        )
        assertEquals("a2", unreadEntryId(entries))
        assertNull(unreadEntryId(listOf(chatEntry("u1", "user"))))
        assertNull(unreadEntryId(emptyList()))
    }

    private fun chatEntry(id: String, kind: String) = Entry(
        id = id,
        seq = 0L,
        botId = "bot",
        rev = 0L,
        kind = kind,
        data = buildJsonObject { put("final", true) },
    )
}
