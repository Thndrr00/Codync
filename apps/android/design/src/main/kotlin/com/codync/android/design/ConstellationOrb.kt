package com.codync.android.design

import androidx.compose.animation.core.withInfiniteAnimationFrameNanos
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.repeatOnLifecycle
import kotlin.math.sin

/** One lifecycle-bound clock for the canvas, read only during layer updates. */
@Composable fun rememberConstellationPhase(moving: Boolean): State<Float> {
    val phase = remember { mutableFloatStateOf(0f) }
    val lifecycle = LocalLifecycleOwner.current.lifecycle
    LaunchedEffect(moving, lifecycle) {
        phase.floatValue = 0f
        if (moving) lifecycle.repeatOnLifecycle(Lifecycle.State.RESUMED) {
            val start = withInfiniteAnimationFrameNanos { it }
            var last = start
            while (true) withInfiniteAnimationFrameNanos { now ->
                if (now - last >= 33_000_000L) {
                    phase.floatValue = ((now - start) / 1e9).toFloat()
                    last = now
                }
            }
        }
    }
    return phase
}

/** Decorative character, activity satellite and unread halo; its parent supplies semantics. */
@Composable fun ConstellationOrb(shape: String, color: String, status: String, unread: Int,
    diameter: Dp, phase: State<Float>, moving: Boolean, modifier: Modifier = Modifier) {
    val attention = status == "needsInput"
    val tint = if (attention) Palette.attention else if (status == "error") Palette.danger else AvatarPalette.tint(color)
    Box(modifier.size(96.dp).clearAndSetSemantics {}, contentAlignment = Alignment.Center) {
        if (attention || unread > 0) Canvas(Modifier.size(diameter + 16.dp)) {
            drawCircle(Brush.radialGradient(listOf(tint.copy(alpha = if (attention) 0.28f else 0.14f), tint.copy(alpha = 0f))))
        }
        Box(Modifier.graphicsLayer {
            val time = if (moving && status == "working") phase.value else 0f
            val pulse = 1f + 0.045f * sin(time * 2f)
            scaleX = pulse; scaleY = pulse
            rotationZ = if (time > 0f) 4f * sin(time * 0.4f) else 0f
            alpha = if (status == "idle") 0.65f else 1f
        }) { CharacterAvatar(shape, color, diameter, status = "idle") }
        if (status == "working" || attention) Box(Modifier.align(Alignment.BottomEnd).padding(8.dp)
            .graphicsLayer { rotationZ = if (moving && !attention) phase.value * 12f % 360f else 0f }) {
            ThinkingOrb(if (attention) ThinkingOrbState.Listening else ThinkingOrbState.Working,
                size = 22.dp, color = tint, animated = false)
        }
        if (attention) Box(Modifier.align(Alignment.TopEnd).padding(6.dp)) {
            Symbol("exclamationmark.circle.fill", size = 18.dp, tint = Palette.attention)
        }
        if (unread > 0) Box(Modifier.align(Alignment.TopStart).padding(4.dp).background(Palette.accentFill, CircleShape)
            .defaultMinSize(minWidth = 22.dp, minHeight = 22.dp).padding(horizontal = 5.dp), contentAlignment = Alignment.Center) {
            Text(if (unread > 99) "99+" else unread.toString(), style = Type.caption2.bold, color = Palette.onAccent)
        }
    }
}
