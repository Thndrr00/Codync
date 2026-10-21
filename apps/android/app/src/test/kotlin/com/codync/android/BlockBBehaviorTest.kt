package com.codync.android

import com.codync.android.design.CodyncHaptic
import com.codync.android.design.RecordingCodyncHaptics
import org.junit.Assert.assertEquals
import org.junit.Test

class BlockBBehaviorTest {

    @Test fun rosterReorderFiresDragStepHapticOnlyWhenRowsActuallySwap() {
        val haptics = RecordingCodyncHaptics()
        val reorder = RosterReorder().apply {
            roster = listOf("a", "b", "c")
            order = roster
            dragged = "a"
            heights["a"] = 100
            heights["b"] = 100
            heights["c"] = 100
        }

        // Small sub-threshold pixel movements must never emit haptics
        reorder.offset = 12f
        reorder.swap("a", haptics)
        reorder.offset = 30f
        reorder.swap("a", haptics)
        reorder.offset = 49f
        reorder.swap("a", haptics)
        assertEquals(emptyList<CodyncHaptic>(), haptics.events)
        assertEquals(listOf("a", "b", "c"), reorder.order)

        // Crossing half the neighbor's height swaps once and emits exactly one DragStep
        reorder.offset = 55f
        reorder.swap("a", haptics)
        assertEquals(listOf("b", "a", "c"), reorder.order)
        assertEquals(listOf(CodyncHaptic.DragStep), haptics.events)
    }
}
