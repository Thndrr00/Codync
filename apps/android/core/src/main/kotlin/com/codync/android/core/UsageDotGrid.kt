package com.codync.android.core

import kotlin.math.roundToInt

/** Geometry shared by usage cards and installed widgets, in density-independent units. */
data class UsageDotGrid(val width: Float, val height: Float, val percent: Double) {
    val columns = maxOf(1, (width / 4).toInt())
    val rows = maxOf(1, (height / 4).toInt())
    val stepX = width / columns
    val stepY = height / rows
    val radius = maxOf(0f, minOf(2.5f, stepX, stepY) / 2)
    val value = if (percent.isFinite()) percent.coerceIn(0.0, 100.0) else 0.0
    val filled = (columns * value / 100).roundToInt()
}
