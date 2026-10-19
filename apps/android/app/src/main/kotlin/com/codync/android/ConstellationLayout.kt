package com.codync.android

import kotlin.math.ceil
import kotlin.math.ln
import kotlin.math.min

/** All dimensions are dp. Cells reserve the maximum halo, pulse and two lines of text. */
internal object ConstellationLayout {
    const val CELL_WIDTH = 112f
    const val CELL_HEIGHT = 160f
    const val MAX_DIAMETER = 76f
    private const val RECENT_WINDOW = 86_400_000L

    data class Point(val x: Float, val y: Float)
    data class Layout(val points: List<Point>, val height: Float)

    fun diameter(lastAt: Long, unread: Int, now: Long): Float {
        val recency = if (lastAt <= 0) 0f else
            (1.0 - (now.toDouble() - lastAt.toDouble()).coerceAtLeast(0.0) / RECENT_WINDOW).coerceIn(0.0, 1.0).toFloat()
        val unreadWeight = (ln(1.0 + unread.coerceAtLeast(0)) / ln(21.0)).coerceAtMost(1.0).toFloat()
        return 48f + 16f * recency + 12f * unreadWeight
    }

    /** Staggered, centered shelves keep list/TalkBack order and never trade readability for density. */
    fun place(count: Int, width: Float, minimumHeight: Float, cellHeight: Float = CELL_HEIGHT): Layout {
        require(count >= 0 && width.isFinite() && width >= CELL_WIDTH)
        require(minimumHeight.isFinite() && minimumHeight >= 0)
        require(cellHeight.isFinite() && cellHeight >= CELL_HEIGHT)
        val columns = (width / CELL_WIDTH).toInt().coerceAtLeast(1)
        val rows = ceil(count.toDouble() / columns).toInt()
        val height = maxOf(minimumHeight, rows * cellHeight + 32f)
        val top = maxOf(16f, (height - rows * cellHeight) / 2f)
        val points = List(count) { index ->
            val row = index / columns
            val inRow = min(columns, count - row * columns)
            val spare = width - inRow * CELL_WIDTH
            val stagger = if (row % 2 == 0) -1f else 1f
            Point(spare / 2f + (index % columns) * CELL_WIDTH + stagger * min(10f, spare / 2f),
                top + row * cellHeight + if (index % 2 == 0) 0f else 10f)
        }
        return Layout(points, height)
    }
}
