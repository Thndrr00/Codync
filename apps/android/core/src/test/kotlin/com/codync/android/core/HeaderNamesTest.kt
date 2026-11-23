package com.codync.android.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class HeaderNamesTest {
    @Test fun `a header name is an http token`() {
        for (good in listOf("Authorization", "X-Api-Key", "X_API_KEY", "X.Api.Key", "X~Key", "X1")) {
            assertTrue("$good is usable", HeaderNames.valid(good))
        }
        for (bad in listOf("Authorization Bearer", "X:Key", "X/Key", "X(Key)", "X@Key", "")) {
            assertFalse("$bad is not usable", HeaderNames.valid(bad))
        }
        assertTrue(HeaderNames.MESSAGE.contains("Authorization"))
        assertTrue(HeaderNames.MESSAGE.contains("Value"))
    }

    @Test fun `the form finds the bad key in a name value box`() {
        assertEquals(null, HeaderNames.invalidKey("Authorization: Bearer sk-1\nX-Api-Key: k"))
        assertEquals(null, HeaderNames.invalidKey("\n\n"))
        assertEquals("Authorization Bearer", HeaderNames.invalidKey("Authorization Bearer: sk-1"))
        assertEquals("X Key", HeaderNames.invalidKey("Authorization: Bearer sk-1\nX Key: k"))
    }
}
