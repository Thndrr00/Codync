package com.codync.android

import kotlinx.serialization.json.JsonPrimitive

internal enum class RootOnboardingStep(val wire: String) { Welcome("welcome"), Pairing("pairing"), Ready("ready") }

internal data class RootOnboardingStepViewed(val step: RootOnboardingStep) : AnalyticsEvent("onboarding_step_viewed") {
    override val properties get() = mapOf("step" to JsonPrimitive(step.wire))
}

internal data class OnboardingAbandoned(val step: RootOnboardingStep) : AnalyticsEvent("onboarding_abandoned") {
    override val properties get() = mapOf("step" to JsonPrimitive(step.wire))
}

/** Closed root steps, persisted across process death. Existing welcome and completion events keep their owners. */
internal class OnboardingTelemetry(
    private val read: () -> String?,
    private val write: (String?) -> Unit,
    private val enabled: () -> Boolean,
    private val capture: (AnalyticsEvent) -> Unit,
) {
    private var shown: RootOnboardingStep? = null
    private var foreground = true

    fun recover() {
        val saved = read()
        val interrupted = RootOnboardingStep.entries.firstOrNull { it.wire == saved }
        if (enabled() && interrupted != null && interrupted != RootOnboardingStep.Ready) {
            capture(OnboardingAbandoned(interrupted))
        }
        clear()
    }

    fun step(step: RootOnboardingStep?) {
        if (shown == step) return
        shown = step
        if (!enabled()) { clear(); return }
        if (!foreground) return
        write(step?.takeUnless { it == RootOnboardingStep.Ready }?.wire)
        // WelcomeView already owns the existing welcome viewed event.
        if (step != null && step != RootOnboardingStep.Welcome) capture(RootOnboardingStepViewed(step))
    }

    fun abandon() {
        if (!foreground) return
        foreground = false
        val step = shown
        if (enabled() && step != null && step != RootOnboardingStep.Ready) capture(OnboardingAbandoned(step))
        clear()
    }

    fun enterForeground() {
        if (foreground) return
        foreground = true
        resume()
    }

    fun resume() {
        val step = shown ?: return
        if (!enabled()) { clear(); return }
        if (!foreground || step == RootOnboardingStep.Ready) return
        write(step.wire)
        if (step != RootOnboardingStep.Welcome) capture(RootOnboardingStepViewed(step))
    }

    fun clear() { write(null) }
}
