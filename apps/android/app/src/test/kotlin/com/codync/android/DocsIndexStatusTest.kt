package com.codync.android

import com.codync.android.core.WireJson
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.decodeFromJsonElement
import kotlinx.serialization.json.put
import org.junit.Assert.*
import org.junit.Test

class DocsIndexStatusTest {
    @Test fun readsTheIndexTheHostReports() {
        val status = WireJson.decodeFromJsonElement<DocsIndexStatus>(buildJsonObject {
            put("folder", "/Users/ana/Projects/site"); put("files", 412); put("bytes", 3_145_728L)
            put("truncated", false); put("builtAt", 1_700_000_000_000L)
        })
        assertEquals(DocsIndexStatus("/Users/ana/Projects/site", 412, 3_145_728L, false), status)
    }

    @Test fun anIndexCutShortSaysSo() {
        val status = WireJson.decodeFromJsonElement<DocsIndexStatus>(buildJsonObject {
            put("folder", "/work/site"); put("files", 4_000); put("bytes", 24L * 1024 * 1024)
            put("truncated", true); put("builtAt", 1L)
        })
        assertTrue(status.truncated)
        assertEquals(4_000, status.files)
    }
}
