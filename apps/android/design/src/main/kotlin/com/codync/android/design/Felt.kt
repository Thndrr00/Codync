package com.codync.android.design

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.kyant.shapes.Capsule

/**
 * Felt badges: the warm, soft look of OpenBot's sidebar, drawn with Codync's own gradients. The
 * amber ball marks a bot that needs you (its cream yarn mark); a count sits in a felt capsule.
 * Both keep the same background ring as the status dots they replace (iOS `FeltBadge.swift`).
 */
object Felt {
    val amberTop = Color(0xFFF5B54E)
    val amberBottom = Color(0xFFC97A0E)
    val cream = Color(0xFFFFF4DA)
}

/** The amber felt ball with its cream yarn mark: a bot that needs you. */
@Composable fun FeltBall(size: Dp) {
    val px = with(androidx.compose.ui.platform.LocalDensity.current) { size.toPx() }
    Box(
        Modifier.size(size)
            .clip(CircleShape)
            .background(Brush.radialGradient(listOf(Felt.amberTop, Felt.amberBottom), center = Offset(px * 0.3f, px * 0.25f), radius = px))
            .semantics { contentDescription = "Needs you" },
        contentAlignment = Alignment.Center,
    ) {
        Canvas(Modifier.size(size * 0.62f)) {
            val w = this.size.width
            val h = this.size.height
            val stroke = Stroke(width = w * 0.16f, cap = StrokeCap.Round)
            fun yarn(from: Offset, c1: Offset, c2: Offset, to: Offset) {
                val path = Path().apply {
                    moveTo(from.x * w, from.y * h)
                    cubicTo(c1.x * w, c1.y * h, c2.x * w, c2.y * h, to.x * w, to.y * h)
                }
                drawPath(path, Felt.cream, style = stroke)
            }
            yarn(Offset(0.16f, 0.34f), Offset(0.34f, 0.02f), Offset(0.66f, 0.98f), Offset(0.84f, 0.64f))
            yarn(Offset(0.2f, 0.72f), Offset(0.4f, 0.96f), Offset(0.6f, 0.04f), Offset(0.8f, 0.3f))
        }
    }
}

/** An unread count in a felt capsule: the accent ink with a soft top light and the same ring. */
@Composable fun FeltCountBadge(count: Int) {
    Box(
        Modifier.height(18.dp).widthIn(min = 18.dp).clip(Capsule())
            .background(Brush.verticalGradient(listOf(Palette.accentFill.copy(alpha = 0.92f), Palette.accentFill.copy(alpha = 0.72f))))
            .padding(horizontal = 6.dp)
            .semantics { contentDescription = "$count unread" },
        contentAlignment = Alignment.Center,
    ) {
        Text("$count", style = Type.caption2.bold, color = Palette.onAccent)
    }
}
