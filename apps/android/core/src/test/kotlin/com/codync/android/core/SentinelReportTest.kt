package com.codync.android.core

import kotlinx.serialization.SerializationException
import kotlinx.serialization.decodeFromString
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class SentinelReportTest {
    @Test fun `actual host issue fields decode without claiming a healthy scan`() {
        val report = WireJson.decodeFromString<SentinelReport>("""
            {"scannedAt":1730000000000,"lookbackMs":604800000,"entriesExamined":42,"issues":[
              {"signature":"retry-signature","kind":"retry_loop","severity":"high","title":"Repeated tool failure",
               "toolName":"shell","botId":"bot-1","firstSeenAt":1729990000000,"lastSeenAt":1729999000000,
               "count":3,"wastedTurns":2,"evidenceEntryId":"entry-1"},
              {"signature":"queue-signature","kind":"queue_lag","severity":"low","title":"Long queue wait",
               "botId":"bot-2","firstSeenAt":1729990000000,"lastSeenAt":1729999000000,
               "count":1,"wastedTurns":0,"evidenceEntryId":"entry-2"}
            ]}
        """.trimIndent())
        assertFalse(report.healthy)
        assertEquals(1730000000000L, report.scannedAt)
        assertEquals(42L, report.entriesExamined)
        assertEquals("Repeated tool failure", report.issues.first().title)
        assertEquals("retry_loop", report.issues.first().kind)
        assertEquals("high", report.issues.first().severity)
        assertEquals("shell", report.issues.first().toolName)
        assertEquals(3L, report.issues.first().count)
        assertEquals(2L, report.issues.first().wastedTurns)
        assertEquals("entry-1", report.issues.first().evidenceEntryId)
        assertNull(report.issues.last().toolName)
    }

    @Test fun `an empty issue list is healthy and additional host fields are tolerated`() {
        val report = WireJson.decodeFromString<SentinelReport>("""
            {"scannedAt":1730000000000,"lookbackMs":604800000,"entriesExamined":0,"issues":[],"futureField":true}
        """.trimIndent())
        assertTrue(report.healthy)
    }

    @Test(expected = SerializationException::class)
    fun `a malformed scan response cannot silently become healthy`() {
        WireJson.decodeFromString<SentinelReport>("""{"healthy":true}""")
    }

    @Test(expected = SerializationException::class)
    fun `an issue using the obsolete category and message shape is rejected`() {
        WireJson.decodeFromString<SentinelReport>("""
            {"scannedAt":1730000000000,"lookbackMs":604800000,"entriesExamined":1,
             "issues":[{"severity":"high","category":"tool_error","message":"Tool failed"}]}
        """.trimIndent())
    }
}
