package com.codync.android.core

/**
 * A short local unlock for sensitive actions on this phone (revealing or replacing secrets). It is only a device
 * guard: the host still decides what an action may do. The grant belongs to one account context, expires, and
 * is revoked when the app leaves the foreground or the account changes, so nothing carries over to another account.
 */
class SensitiveAuthorization(private val lifetimeMillis: Long = DEFAULT_LIFETIME_MILLIS) {
    private var contextId: String? = null
    private var expiresAt = 0L

    @Synchronized fun grant(context: String, now: Long) { contextId = context; expiresAt = now + lifetimeMillis }
    @Synchronized fun isValid(context: String, now: Long): Boolean = contextId == context && now < expiresAt
    @Synchronized fun revoke() { contextId = null; expiresAt = 0 }

    companion object { const val DEFAULT_LIFETIME_MILLIS = 60_000L }
}
