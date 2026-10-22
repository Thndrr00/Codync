package com.codync.android.core

import org.junit.Assert.*
import org.junit.Test

class ShareIntakeTest {
    @Test fun `only text, images and pdfs are accepted`() {
        assertTrue(ShareIntake.accepts("text/plain"))
        assertTrue(ShareIntake.accepts("image/png"))
        assertTrue(ShareIntake.accepts("application/pdf; charset=binary"))
        assertFalse(ShareIntake.accepts("application/zip"))
        assertFalse(ShareIntake.accepts(null))
    }

    @Test fun `a subject leads the shared text unless it repeats it`() {
        assertEquals("Title\n\nBody", ShareIntake.draftText("Title", "Body"))
        assertEquals("Body", ShareIntake.draftText("Body", " Body "))
        assertEquals("", ShareIntake.draftText(null, null))
    }

    @Test fun `shared text follows what the user already typed`() {
        assertEquals("typed\n\nshared", ShareIntake.merge("typed ", "shared"))
        assertEquals("shared", ShareIntake.merge("", "shared"))
        assertEquals("typed", ShareIntake.merge("typed", ""))
    }

    @Test fun `a merge never passes what one draft can save`() {
        val merged = ShareIntake.merge("", "漢".repeat(ShareIntake.MAX_TEXT))
        assertTrue(merged.toByteArray().size <= ShareIntake.MAX_DRAFT_BYTES)
        // The cut lands between characters, never inside one.
        assertTrue(merged.all { it == '漢' })
    }
}
