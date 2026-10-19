package com.codync.android

import org.junit.Assert.*
import org.junit.Test

class ConstellationLayoutTest {
    @Test fun sizeIsBoundedAndRewardsRecentActivityAndUnread() {
        val now = 2_000_000_000L
        val quiet = ConstellationLayout.diameter(0, 0, now)
        assertEquals(48f, quiet, 0f)
        assertTrue(ConstellationLayout.diameter(now, 0, now) > quiet)
        assertTrue(ConstellationLayout.diameter(now, 5, now) > ConstellationLayout.diameter(now, 1, now))
        assertEquals(ConstellationLayout.MAX_DIAMETER, ConstellationLayout.diameter(Long.MAX_VALUE, Int.MAX_VALUE, now), 0.001f)
        assertEquals(quiet, ConstellationLayout.diameter(1, -10, now), 0f)
        assertTrue(ConstellationLayout.diameter(now - 3_600_000, 0, now) > ConstellationLayout.diameter(now - 86_400_000, 0, now))
    }

    @Test fun cellsStayInsideCanvasAndNeverOverlap() {
        for (cellHeight in listOf(160f, 208f, 256f)) {
            for (count in listOf(0, 1, 2, 30, 100)) for (width in listOf(112f, 280f, 360f, 800f)) {
                val extent = cellHeight - 10f
                val result = ConstellationLayout.place(count, width, 500f, cellHeight)
                assertEquals(count, result.points.size)
                assertEquals(result, ConstellationLayout.place(count, width, 500f, cellHeight))
                result.points.forEachIndexed { index, point ->
                    assertTrue(point.x >= 0f && point.x + ConstellationLayout.CELL_WIDTH <= width)
                    assertTrue(point.y >= 0f && point.y + extent <= result.height)
                    result.points.take(index).forEach { other ->
                        assertTrue(kotlin.math.abs(point.x - other.x) >= ConstellationLayout.CELL_WIDTH ||
                            kotlin.math.abs(point.y - other.y) >= extent)
                    }
                }
            }
        }
    }

    @Test fun oneAndTwoBotsRemainProminentAndCentered() {
        val one = ConstellationLayout.place(1, 360f, 600f)
        assertTrue(kotlin.math.abs(one.points.single().x + 56f - 180f) <= 10f)
        val two = ConstellationLayout.place(2, 360f, 600f)
        assertEquals(600f, two.height, 0f)
        assertTrue(two.points.all { it.y in 200f..300f })
    }
}
