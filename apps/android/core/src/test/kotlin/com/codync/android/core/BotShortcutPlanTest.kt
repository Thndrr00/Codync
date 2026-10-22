package com.codync.android.core

import org.junit.Assert.*
import org.junit.Test

class BotShortcutPlanTest {
    private val computer = "A".repeat(22)

    @Test fun `only visible pinned bots become shortcuts, up to the launcher limit`() {
        val bots = listOf(
            Bot("a", "Alpha", pinned = true), Bot("b", "Beta"), Bot("c", "Gamma", pinned = true, hidden = true),
            Bot("d", "Delta", pinned = true), Bot("e", "Echo", pinned = true), Bot("f", "Foxtrot", pinned = true),
        )
        val plan = BotShortcutPlan.plan("local", computer, bots, limit = 2)
        assertEquals(listOf("a", "d"), plan.map { it.botId })
    }

    @Test fun `a shortcut id names the account, computer and bot`() {
        val plan = BotShortcutPlan.plan("local", computer, listOf(Bot("a", "Alpha", pinned = true)), limit = 4)
        assertEquals("launch:local/$computer/a", plan.single().id)
        assertTrue(plan.single().id.startsWith(BotShortcutPlan.PREFIX))
    }

    @Test fun `a zero limit publishes nothing`() {
        assertTrue(BotShortcutPlan.plan("local", computer, listOf(Bot("a", "Alpha", pinned = true)), limit = 0).isEmpty())
    }
}
