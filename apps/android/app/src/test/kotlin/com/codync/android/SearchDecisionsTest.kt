package com.codync.android

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class SearchDecisionsTest {
    @Test fun `reduce motion and the none setting switch search rows off`() {
        assertEquals(SearchMotion.Fade, searchMotion(SearchMotionStyle.Fade, false))
        // Blur is an iOS veil; Android keeps the fade (OpenBot 004's motion palette).
        assertEquals(SearchMotion.Fade, searchMotion(SearchMotionStyle.Blur, false))
        assertEquals(SearchMotion.None, searchMotion(SearchMotionStyle.None, false))
        assertEquals(SearchMotion.None, searchMotion(SearchMotionStyle.Fade, true))
        assertEquals(SearchMotion.None, searchMotion(SearchMotionStyle.Blur, true))
    }

    @Test fun `the searching spinner waits for the host and never flashes on a fast answer`() {
        assertFalse(showSpinner(null, 1_000L))
        assertFalse(showSpinner(800L, 999L))
        assertFalse(showSpinner(800L, 999L, delayMs = 300L))
        assertTrue(showSpinner(800L, 1_000L))
        assertTrue(showSpinner(800L, 1_500L, delayMs = 300L))
    }

    @Test fun `prior hits stay while an edited query loads and only a cleared field starts empty`() {
        assertTrue(hitsAfter(queryChanged = false, cleared = false))
        assertTrue(hitsAfter(queryChanged = true, cleared = false))
        assertFalse(hitsAfter(queryChanged = true, cleared = true))
        assertTrue(hitsAfter(queryChanged = false, cleared = true))
    }

    @Test fun `the preference labels name the three settings options`() {
        assertEquals("Animate search results", SearchMotionStyle.TITLE)
        assertEquals(listOf("Blur", "Fade", "None"), SearchMotionStyle.entries.map { it.label })
        assertEquals(listOf("blur", "fade", "none"), SearchMotionStyle.entries.map { it.wire })
    }
}
