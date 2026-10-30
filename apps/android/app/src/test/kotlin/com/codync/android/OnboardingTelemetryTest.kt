package com.codync.android

import org.junit.Assert.*
import org.junit.Test

class OnboardingTelemetryTest {
    private class Fixture {
        var saved: String? = null
        var enabled = true
        val events = mutableListOf<AnalyticsEvent>()
        fun tracker() = OnboardingTelemetry({ saved }, { saved = it }, { enabled }, { events.add(it) })
    }

    @Test fun `process death and backgrounding report each unfinished attempt once`() {
        val fixture = Fixture()
        fixture.saved = "pairing"
        val tracker = fixture.tracker()
        tracker.recover()
        assertEquals(listOf(OnboardingAbandoned(RootOnboardingStep.Pairing)), fixture.events)
        fixture.tracker().recover()
        assertEquals(1, fixture.events.size)
        tracker.step(RootOnboardingStep.Pairing)
        tracker.step(RootOnboardingStep.Pairing)
        assertEquals("pairing", fixture.saved)
        tracker.abandon()
        tracker.abandon()
        assertEquals(2, fixture.events.filterIsInstance<OnboardingAbandoned>().size)
        assertNull(fixture.saved)
        tracker.enterForeground()
        tracker.enterForeground()
        assertEquals("pairing", fixture.saved)
        assertEquals(2, fixture.events.filterIsInstance<RootOnboardingStepViewed>().size)
    }

    @Test fun `ready never abandons completed setup or duplicates the existing completion event`() {
        val fixture = Fixture()
        val tracker = fixture.tracker()
        tracker.step(RootOnboardingStep.Pairing)
        tracker.step(RootOnboardingStep.Ready)
        tracker.step(RootOnboardingStep.Ready)
        tracker.abandon()
        fixture.tracker().recover()
        tracker.enterForeground()
        assertNull(fixture.saved)
        assertEquals(1, fixture.events.filterIsInstance<RootOnboardingStepViewed>().count { it.step == RootOnboardingStep.Ready })
        assertTrue(fixture.events.none { it is OnboardingAbandoned || it.name == "onboarding_completed" })
    }

    @Test fun `opt out clears recovery state and suppresses step and abandonment events`() {
        val fixture = Fixture()
        val tracker = fixture.tracker()
        tracker.step(RootOnboardingStep.Pairing)
        val before = fixture.events.toList()
        fixture.enabled = false
        tracker.clear()
        tracker.abandon()
        fixture.tracker().recover()
        tracker.enterForeground()
        tracker.step(RootOnboardingStep.Welcome)
        assertNull(fixture.saved)
        assertEquals(before, fixture.events)
    }

    @Test fun `welcome view keeps its existing viewed event and only closed root values are captured`() {
        val fixture = Fixture()
        fixture.saved = "unexpected private text"
        val tracker = fixture.tracker()
        tracker.recover()
        tracker.step(RootOnboardingStep.Welcome)
        tracker.step(RootOnboardingStep.Welcome)
        assertTrue(fixture.events.isEmpty())
        tracker.abandon()
        assertEquals(listOf(OnboardingAbandoned(RootOnboardingStep.Welcome)), fixture.events)
        assertEquals("{step=\"welcome\"}", fixture.events.single().properties.toString())
    }
}
