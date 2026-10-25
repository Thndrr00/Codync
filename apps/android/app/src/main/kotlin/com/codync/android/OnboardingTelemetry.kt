package com.codync.android

import android.content.SharedPreferences

/** Closed step names only. A process ending mid-step is recorded at the next start. */
internal object OnboardingTelemetry {
    enum class Step(val wire: String) { Welcome("welcome"), Pairing("pairing"), Ready("ready") }
    private var prefs: SharedPreferences? = null
    private var shown: Step? = null
    fun start(stored: SharedPreferences) {
        prefs = stored
        val interrupted = Step.entries.firstOrNull { it.wire == stored.getString("onboarding_step", null) }
        if (interrupted != null && interrupted != Step.Ready && Analytics.enabled) {
            Analytics.capture(AnalyticsEvent.OnboardingAbandoned, mapOf("step" to interrupted.wire))
        }
        stored.edit().remove("onboarding_step").commit()
    }
    fun step(step: Step?) {
        if (shown == step) return
        shown = step
        if (!Analytics.enabled) { clear(); return }
        prefs?.edit()?.putString("onboarding_step", step?.wire)?.commit()
        if (step != null) Analytics.capture(AnalyticsEvent.OnboardingStepViewed, mapOf("step" to step.wire))
    }
    fun abandon() {
        val step = shown ?: return
        if (step != Step.Ready && Analytics.enabled) {
            Analytics.capture(AnalyticsEvent.OnboardingAbandoned, mapOf("step" to step.wire))
            prefs?.edit()?.remove("onboarding_step")?.commit()
        }
    }
    fun foreground() {
        val step = shown ?: return
        if (step != Step.Ready && Analytics.enabled) {
            prefs?.edit()?.putString("onboarding_step", step.wire)?.commit()
            Analytics.capture(AnalyticsEvent.OnboardingStepViewed, mapOf("step" to step.wire))
        }
    }
    fun clear() { prefs?.edit()?.remove("onboarding_step")?.commit() }
}
