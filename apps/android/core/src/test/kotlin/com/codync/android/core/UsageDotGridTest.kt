package com.codync.android.core

import org.junit.Assert.*
import org.junit.Test

class UsageDotGridTest {
    @Test fun `zero usage stays empty and invalid readings cannot overflow`() {
        for (value in listOf(0.0, -3.0, Double.NaN, Double.POSITIVE_INFINITY, Double.NEGATIVE_INFINITY)) {
            assertEquals(0, UsageDotGrid(60f, 8f, value).filled)
        }
        assertEquals(15, UsageDotGrid(60f, 8f, 125.0).filled)
        assertEquals(8, UsageDotGrid(60f, 8f, 50.0).filled)
    }

    @Test fun `dots stay round and inside small containers`() {
        for ((width, height) in listOf(0f to 0f, 1f to 2f, 28f to 8f, 103.5f to 13f)) {
            val grid = UsageDotGrid(width, height, 62.0)
            assertTrue(grid.radius <= minOf(grid.stepX, grid.stepY) / 2)
            assertTrue((grid.columns - 0.5f) * grid.stepX + grid.radius <= width)
            assertTrue((grid.rows - 0.5f) * grid.stepY + grid.radius <= height)
        }
    }
}
