package com.codync.android

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class StepErrorHintTest {
    @Test fun `a failed step names what to do from its error`() {
        assertEquals("Sign in again in Settings, then run this step.", StepErrorHint.hint(StepErrorHint.kind("Couldn't sign in to the account")))
        assertEquals("Give every path into this step the field, or mark it optional.", StepErrorHint.hint(StepErrorHint.kind("Field is required")))
        assertTrue(StepErrorHint.hint(StepErrorHint.kind("Something else went wrong")).contains("run it again"))
    }

    @Test fun `a step error kind covers the known failures`() {
        assertEquals("signin", StepErrorHint.kind("auth failed"))
        assertEquals("model", StepErrorHint.kind("unknown model"))
        assertEquals("limit", StepErrorHint.kind("quota reached"))
        assertEquals("field", StepErrorHint.kind("no answer"))
        assertEquals("other", StepErrorHint.kind("mystery"))
    }
}
