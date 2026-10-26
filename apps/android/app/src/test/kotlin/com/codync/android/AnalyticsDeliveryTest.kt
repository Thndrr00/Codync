package com.codync.android

import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import org.junit.Assert.*
import org.junit.Test

class AnalyticsDeliveryTest {
    private class Storage {
        var saved: String? = null
        var writable = true
        fun queue(restore: Boolean = true, capacity: Int = 500) = AnalyticsQueue(
            { saved }, { value -> if (writable) { saved = value; true } else false }, restore, capacity,
        )
    }

    private fun event(id: String): JsonObject = buildJsonObject {
        put("uuid", id)
        put("event", "app_opened")
        put("properties", buildJsonObject { put("\$session_id", "original-session") })
    }

    @Test fun `queued events survive a process restart with their original identities and session metadata`() {
        val storage = Storage()
        val first = event("00000000-0000-4000-8000-000000000001")
        assertTrue(storage.queue().add(first))
        val restarted = storage.queue()
        assertEquals(listOf(first), restarted.batch(20))
        // A retry never regenerates an event UUID or replaces the session in which it happened.
        assertEquals(restarted.batch(20), storage.queue().batch(20))
    }

    @Test fun `late acknowledgements preserve newer events and the bounded queue survives restart`() {
        val storage = Storage()
        val queue = storage.queue(capacity = 3)
        queue.add(event("first"))
        val sent = queue.batch(20)
        for (id in listOf("second", "third", "fourth")) queue.add(event(id))
        queue.acknowledge(sent)
        assertEquals(listOf("second", "third", "fourth").map(::event), storage.queue(capacity = 3).batch(20))
    }

    @Test fun `events are not dispatched when persistence fails`() {
        val storage = Storage()
        val queue = storage.queue()
        storage.writable = false
        assertFalse(queue.add(event("pending")))
        assertTrue(queue.batch(20).isEmpty())
        storage.writable = true
        assertEquals(listOf(event("pending")), queue.batch(20))
        assertEquals(listOf(event("pending")), storage.queue().batch(20))
    }

    @Test fun `opting out cancels active delivery rejects stale dispatch and clears persisted pending events`() {
        val storage = Storage()
        val queue = storage.queue()
        queue.add(event("pending"))
        val consent = AnalyticsConsent(true)
        val old = consent.version
        var cancellations = 0
        assertTrue(consent.register(old) { cancellations++ })
        consent.change(false)
        queue.clear()
        assertEquals(1, cancellations)
        assertFalse(consent.allows(old))
        assertFalse(consent.register(old) { fail("stale delivery must not start") })
        assertTrue(storage.queue().batch(20).isEmpty())
        consent.change(true)
        assertFalse(consent.allows(old))
        assertTrue(consent.allows(consent.version))
    }

    @Test fun `disabled startup wipes a saved queue and a late request cannot clear a newer cancellation`() {
        val storage = Storage()
        storage.queue().add(event("pending"))
        assertEquals(0, storage.queue(restore = false).size)
        assertTrue(storage.queue().batch(20).isEmpty())
        val consent = AnalyticsConsent(true)
        val first = consent.version
        assertTrue(consent.register(first) {})
        consent.change(false)
        consent.change(true)
        var cancelled = false
        assertTrue(consent.register(consent.version) { cancelled = true })
        consent.finished(first)
        consent.change(false)
        assertTrue(cancelled)
    }

    @Test fun `transient responses and redirects retain their events`() {
        for (status in listOf(301, 307, 408, 429, 500, 503)) assertFalse("$status", analyticsDeliveryAccepted(status))
        for (status in listOf(200, 202, 204, 400, 422)) assertTrue("$status", analyticsDeliveryAccepted(status))
    }
}
