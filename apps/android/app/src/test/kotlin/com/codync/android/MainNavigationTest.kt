package com.codync.android

import androidx.compose.ui.unit.dp
import com.codync.android.core.Bot
import com.codync.android.core.Computer
import com.codync.android.core.HostRoute
import com.codync.android.core.LinkState
import com.codync.android.design.NavBarStyle
import com.codync.android.design.TabBadgeTone
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class MainNavigationTest {
    private val pairedComputer = Computer("comp-1", "Mac Studio", "pubkey")

    @Test fun unreadBadgeSumsVisibleBotsAndCapsVisualLabelAt99Plus() {
        val state = AppState(
            computers = listOf(pairedComputer),
            connection = LinkState.Ready(HostRoute.Direct),
            actualConnection = LinkState.Ready(HostRoute.Direct),
            bots = listOf(
                Bot("b1", "Alpha", unread = 80),
                Bot("b2", "Beta", unread = 55),
                Bot("b3", "Hidden", unread = 40, hidden = true),
                Bot("b4", "Deleted", unread = 10, deleted = true),
            ),
        )
        val badges = navigationBadges(state)
        val botsBadge = badges.bots
        assertNotNull(botsBadge)
        assertEquals("99+", botsBadge?.visualText)
        assertEquals("135 unread", botsBadge?.accessibilityLabel)
        assertEquals(TabBadgeTone.Accent, botsBadge?.tone)
    }

    @Test fun stateBadgeDeduplicatesGroupAndMemberActivityAndPrioritizesApproval() {
        val member1 = Bot("m1", "Coder", status = "needsInput")
        val member2 = Bot("m2", "Reviewer", status = "working")
        val group = Bot(
            id = "g1",
            name = "Team",
            kind = "group",
            status = "needsInput",
            members = listOf("m1", "m2"),
        )
        val state = AppState(
            computers = listOf(pairedComputer),
            connection = LinkState.Ready(HostRoute.Direct),
            actualConnection = LinkState.Ready(HostRoute.Direct),
            bots = listOf(member1, member2, group),
        )
        val badges = navigationBadges(state)
        val stateBadge = badges.state
        assertNotNull(stateBadge)
        assertEquals("1", stateBadge?.visualText)
        assertEquals("1 needs your approval, 1 working", stateBadge?.accessibilityLabel)
        assertEquals(TabBadgeTone.Warning, stateBadge?.tone)
        assertTrue(stateBadge?.pulse == true)
    }

    @Test fun offlineConnectionSuppressesCachedWorkBadgesAndMarksComputerOffline() {
        val state = AppState(
            computers = listOf(pairedComputer),
            connection = LinkState.ComputerOffline(lastSeenAt = null),
            actualConnection = LinkState.ComputerOffline(lastSeenAt = null),
            bots = listOf(
                Bot("m1", "Coder", status = "needsInput", unread = 3),
                Bot("m2", "Runner", status = "working"),
            ),
        )
        val badges = navigationBadges(state)
        assertEquals("3", badges.bots?.visualText)
        assertNull(badges.state)
        assertEquals(TabBadgeTone.Offline, badges.computers?.tone)
        assertNull(badges.computers?.visualText)
    }

    @Test fun barScrollVisibilityAppliesHysteresisAndRespectsTouchAndSuppressionGuards() {
        val visibility = BarScrollVisibility(hideThresholdPx = 56f, showThresholdPx = 28f)
        assertTrue(visibility.visible)

        // Programmatic scroll or scroll while not allowed never hides the bar.
        visibility.touching = true
        visibility.onScrollDelta(-30f)
        assertTrue(visibility.visible)
        // Direction reversal resets accumulated delta before threshold is reached.
        visibility.onScrollDelta(5f)
        visibility.onScrollDelta(-30f)
        assertTrue(visibility.visible)
        visibility.onScrollDelta(-30f)
        assertFalse(visibility.visible)

        // Small upward jitter does not show the bar until showThresholdPx is reached.
        visibility.onScrollDelta(15f)
        assertFalse(visibility.visible)
        visibility.onScrollDelta(15f)
        assertTrue(visibility.visible)

        // Suppressed auto-hide or focus immediately restores and locks visibility.
        visibility.onScrollDelta(-80f)
        assertFalse(visibility.visible)
        visibility.canAutoHide = false
        visibility.onScrollDelta(-80f)
        assertTrue(visibility.visible)
    }

    @Test fun navBarStyleAndAdaptiveWindowBreakpointsRoundTrip() {
        for (style in NavBarStyle.entries) {
            assertEquals(style, NavBarStyle.fromId(style.id))
        }
        assertEquals(NavBarStyle.FloatingCapsule, NavBarStyle.fromId("unknown"))
        assertEquals(NavBarStyle.FloatingCapsule, NavBarStyle.fromId(null))

        assertFalse(isExpandedWindow(411.dp, 891.dp))
        assertFalse(isExpandedWindow(700.dp, 320.dp))
        assertTrue(isExpandedWindow(600.dp, 480.dp))
        assertTrue(isExpandedWindow(840.dp, 900.dp))
    }
}
