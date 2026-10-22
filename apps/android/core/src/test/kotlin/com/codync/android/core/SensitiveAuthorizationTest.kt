package com.codync.android.core

import org.junit.Assert.*
import org.junit.Test

class SensitiveAuthorizationTest {
    @Test fun `a grant lasts only its lifetime`() {
        val auth = SensitiveAuthorization(lifetimeMillis = 1_000)
        assertFalse(auth.isValid("a", 0))
        auth.grant("a", now = 100)
        assertTrue(auth.isValid("a", 1_099))
        assertFalse(auth.isValid("a", 1_100))
    }

    @Test fun `another account never inherits the unlock`() {
        val auth = SensitiveAuthorization()
        auth.grant("a", 0)
        assertFalse(auth.isValid("b", 1))
    }

    @Test fun `revoking removes the unlock`() {
        val auth = SensitiveAuthorization()
        auth.grant("a", 0)
        auth.revoke()
        assertFalse(auth.isValid("a", 1))
    }
}
