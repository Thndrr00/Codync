package com.codync.android

import com.codync.android.core.Bot
import org.junit.Assert.assertEquals
import org.junit.Test

class ArchivedGroupsTest {
    @Test fun anArchivedGroupLeavesTheOpenRosterAndIsOfferedToRestore() {
        val bots = listOf(
            Bot("open", kind = "group"),
            Bot("archived", name = "Beta", kind = "group", archived = true),
            Bot("hidden", kind = "group", archived = true, hidden = true),
            Bot("agent"),
        )
        assertEquals(listOf("open", "agent"), roster(bots).map(Bot::id))
        assertEquals(listOf("archived"), archivedGroups(bots).map(Bot::id))
    }

    @Test fun anAgentIsNeverTreatedAsAnArchivedGroup() {
        val bots = listOf(Bot("agent", archived = true))
        assertEquals(listOf("agent"), roster(bots).map(Bot::id))
        assertEquals(emptyList<Bot>(), archivedGroups(bots))
    }
}
