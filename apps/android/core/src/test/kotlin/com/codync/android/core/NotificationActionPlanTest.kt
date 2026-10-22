package com.codync.android.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class NotificationActionPlanTest {
    private val scope = "a".repeat(64)
    private val computer = "B".repeat(22)
    private val nonce = "3f2b1c9e-0a1b-4c2d-8e3f-123456789abc"

    @Test fun `a reply keeps its trimmed text and belongs to one conversation`() {
        val action = NotificationActionPlan.reply(nonce, scope, computer, "bot-1", "  Hello  ")
        assertNotNull(action)
        assertEquals("Hello", action?.reply)
        assertFalse(action!!.isApproval)
    }

    @Test fun `an empty or oversized reply is dropped`() {
        assertNull(NotificationActionPlan.reply(nonce, scope, computer, "bot-1", "   "))
        assertNull(NotificationActionPlan.reply(nonce, scope, computer, "bot-1", "x".repeat(NotificationActionPlan.MAX_REPLY + 1)))
        assertNotNull(NotificationActionPlan.reply(nonce, scope, computer, "bot-1", "x".repeat(NotificationActionPlan.MAX_REPLY)))
    }

    @Test fun `an approval needs a real card and option id`() {
        val action = NotificationActionPlan.approval(nonce, scope, computer, "bot-1", "entry_1", "allow-once")
        assertNotNull(action)
        assertTrue(action!!.isApproval)
        assertEquals("allow-once", action.optionId)
        assertNull(NotificationActionPlan.approval(nonce, scope, computer, "bot-1", "", "allow-once"))
        assertNull(NotificationActionPlan.approval(nonce, scope, computer, "bot-1", "entry_1", "allow once"))
    }

    @Test fun `a malformed target or nonce is dropped`() {
        assertNull(NotificationActionPlan.reply("short", scope, computer, "bot-1", "Hi"))
        assertNull(NotificationActionPlan.reply(nonce, "", computer, "bot-1", "Hi"))
        assertNull(NotificationActionPlan.reply(nonce, scope, "bad id", "bot-1", "Hi"))
        assertNull(NotificationActionPlan.reply(nonce, scope, computer, "bot/1", "Hi"))
    }

    @Test fun `a re-posted alert names the same work and another reply names its own`() {
        val first = NotificationActionPlan.workName(nonce)
        assertEquals(first, NotificationActionPlan.workName(nonce))
        assertTrue(first.startsWith(NotificationActionPlan.WORK_PREFIX))
        assertNotEquals(first, NotificationActionPlan.workName(NotificationActionPlan.freshNonce()))
    }
}
