package com.codync.android.core

import kotlinx.serialization.decodeFromString
import kotlinx.serialization.encodeToString
import org.junit.Assert.*
import org.junit.Test

class ScreenCandidateTest {
    @Test fun `older hosts continue using complete SDP and newer hosts opt in`() {
        val legacy = WireJson.decodeFromString<ScreenConnection>("""{"session":"s","iceServers":[],"expiresAt":1000}""")
        assertFalse(legacy.trickle)
        val newer = WireJson.decodeFromString<ScreenConnection>("""{"session":"s","iceServers":[],"expiresAt":1000,"trickle":true}""")
        assertTrue(newer.trickle)
    }

    @Test fun `candidate byte limits match the host for multibyte text`() {
        val candidate = ScreenCandidate("candidate", "candidate:" + "é".repeat(2043), 0, "video")
        candidate.validate()
        assertEquals(4096, candidate.candidate!!.toByteArray(Charsets.UTF_8).size)
        assertThrows(IllegalArgumentException::class.java) { candidate.copy(candidate = candidate.candidate + "é").validate() }
    }

    @Test fun `invalid SDP and media identifiers are rejected before entering signaling`() {
        val candidate = ScreenCandidate("candidate", "candidate:1 1 UDP 1 127.0.0.1 9 typ host", 0, "0")
        for (invalid in listOf(candidate.copy(candidate = null), candidate.copy(candidate = "untrusted"),
            candidate.copy(candidate = candidate.candidate + "\r\n"), candidate.copy(sdpMLineIndex = null),
            candidate.copy(sdpMLineIndex = -1), candidate.copy(sdpMLineIndex = 17),
            candidate.copy(sdpMid = "é".repeat(129)), candidate.copy(type = "unknown"))) {
            assertThrows(IllegalArgumentException::class.java) { invalid.validate() }
        }
        candidate.copy(sdpMLineIndex = 16, sdpMid = "é".repeat(128)).validate()
        assertEquals(candidate, WireJson.decodeFromString<ScreenCandidate>(WireJson.encodeToString(candidate)))
    }
}
