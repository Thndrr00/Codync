package com.codync.android.design

import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.dp
import org.junit.Assert.assertEquals
import org.junit.Test

/** The sliding pill lands on the measured tab, even when labels have different widths. */
class TabPillGeometryTest {

    @Test fun the_pill_starts_at_the_first_measured_tab() {
        val bounds = mapOf("a" to DpSize(80.dp, 32.dp), "b" to DpSize(140.dp, 32.dp), "c" to DpSize(60.dp, 32.dp))
        val options = listOf("a", "b", "c")

        assertEquals(0.dp, tabPillOffset(bounds, options, 0, 2.dp))
        assertEquals(82.dp, tabPillOffset(bounds, options, 1, 2.dp))
        assertEquals(224.dp, tabPillOffset(bounds, options, 2, 2.dp))
    }

    @Test fun a_tab_not_measured_yet_does_not_move_the_pill_past_its_neighbours() {
        val bounds = mapOf("a" to DpSize(80.dp, 32.dp))
        assertEquals(0.dp, tabPillOffset(bounds, listOf("a", "b", "c"), -1, 2.dp))
        assertEquals(82.dp, tabPillOffset(bounds, listOf("a", "b", "c"), 1, 2.dp))
    }
}
