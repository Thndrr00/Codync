package com.codync.android

import android.os.SystemClock
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import androidx.fragment.app.FragmentActivity
import com.codync.android.core.Entry
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.TimeoutCancellationException
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeout
import kotlin.coroutines.resume

/**
 * Approvals a person has to unlock: a command or a file change on their computer takes a thumb (or the device
 * credential) before it reaches the host. Denying never does — refusing is always one tap.
 *
 * Two guards, in the spirit of Coucou's Android app:
 * - the thumb is asked only for what changes the computer (`execute`, `edit`, `delete`, `move`), so reading
 *   files or fetching a page stays out of the way;
 * - taps in the first 600 ms after the screen lights up are ignored: that is the pocket, not the person.
 *
 * A device with no biometry enrolled is never locked out of its own approvals: the tap goes through. The same
 * holds while no Codync screen is on the display, which is where approvals are tapped and spoken (voice calls).
 */
internal object Approvals {
    private val changesTheComputer = setOf("execute", "edit", "delete", "move")
    private const val POCKET_GUARD_MS = 600L
    /** A prompt nobody answers must not leave the card unanswerable. */
    private const val PROMPT_TIMEOUT_MS = 30_000L

    @Volatile private var screenLitAt = 0L

    /** The screen the person is looking at, while one is attached. */
    @Volatile private var current: FragmentActivity? = null

    /** The screen just turned on: the next half second of taps is not a decision. */
    fun screenLit() {
        screenLitAt = SystemClock.elapsedRealtime()
    }

    fun attach(activity: FragmentActivity) {
        current = activity
    }

    fun detach(activity: FragmentActivity) {
        if (current === activity) current = null
    }

    fun isSensitive(entry: Entry): Boolean = entry.data.text("toolKind") in changesTheComputer

    /** For the store, which has no screen of its own: the answer is gated on the one that is showing. */
    suspend fun confirmCurrent(entry: Entry, optionId: String?): Boolean {
        val activity = current ?: return true
        return confirm(activity, entry, optionId)
    }

    /** A deny is always one tap; an allow of something that changes the computer waits for the person. */
    suspend fun confirm(activity: FragmentActivity, entry: Entry, optionId: String?): Boolean {
        if (SystemClock.elapsedRealtime() - screenLitAt < POCKET_GUARD_MS) return false
        if (optionId == null || !optionId.startsWith("allow") || !isSensitive(entry)) return true
        val allowed = BiometricManager.Authenticators.BIOMETRIC_WEAK or
            BiometricManager.Authenticators.DEVICE_CREDENTIAL
        val status = BiometricManager.from(activity).canAuthenticate(allowed)
        // Nothing enrolled on this phone: its owner approves as before. A lockout or missing hardware is
        // "can't confirm right now", so the card stays pending instead of letting the tap through.
        if (status == BiometricManager.BIOMETRIC_ERROR_NONE_ENROLLED ||
            status == BiometricManager.BIOMETRIC_ERROR_NO_HARDWARE
        ) {
            return true
        }
        if (status != BiometricManager.BIOMETRIC_SUCCESS) return false
        return try {
            withTimeout(PROMPT_TIMEOUT_MS) {
                withContext(Dispatchers.Main.immediate) {
                    suspendCancellableCoroutine { cont ->
                        val prompt = BiometricPrompt(
                            activity,
                            ContextCompat.getMainExecutor(activity),
                            object : BiometricPrompt.AuthenticationCallback() {
                                override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                                    if (cont.isActive) cont.resume(true)
                                }

                                override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                                    if (cont.isActive) cont.resume(false)
                                }
                            },
                        )
                        cont.invokeOnCancellation { prompt.cancelAuthentication() }
                        val info = BiometricPrompt.PromptInfo.Builder()
                            .setTitle("Approve ${entry.data.text("title").orEmpty()}")
                            .setSubtitle("This changes files or runs a command on your computer")
                            .setAllowedAuthenticators(allowed)
                            .build()
                        prompt.authenticate(info)
                    }
                }
            }
        } catch (_: TimeoutCancellationException) {
            false
        }
    }
}
