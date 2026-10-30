package com.codync.android.design

import android.animation.ValueAnimator
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.keyframes
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.offset
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.BlendMode
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.CompositingStrategy
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.IntOffset
import kotlin.math.roundToInt

/**
 * Transitions.dev-inspired shimmer sweep modifier for active thinking/working labels (Idea 16).
 * Respects system animation disabling (`ValueAnimator.areAnimatorsEnabled()`).
 */
@Composable
fun Modifier.shimmerEffect(active: Boolean = true): Modifier {
    if (!active || !ValueAnimator.areAnimatorsEnabled()) return this
    val transition = rememberInfiniteTransition(label = "shimmerSweep")
    val progress by transition.animateFloat(
        initialValue = -0.6f,
        targetValue = 1.6f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 1900, easing = LinearEasing),
            repeatMode = RepeatMode.Restart,
        ),
        label = "shimmerProgress",
    )
    return this
        .graphicsLayer(compositingStrategy = CompositingStrategy.Offscreen)
        .drawWithContent {
            drawContent()
            val width = size.width.coerceAtLeast(1f)
            val center = width * progress
            val span = width * 0.45f
            val brush = Brush.linearGradient(
                colors = listOf(
                    Color.White.copy(alpha = 0.55f),
                    Color.White,
                    Color.White.copy(alpha = 0.55f),
                ),
                start = Offset(center - span, 0f),
                end = Offset(center + span, size.height),
            )
            drawRect(brush = brush, blendMode = BlendMode.DstIn)
        }
}

/**
 * Idea 26: Spinning odometer-style vertical number counter for token/item counts.
 */
@Composable
fun SpinningCounter(
    value: Int,
    modifier: Modifier = Modifier,
    style: TextStyle = Type.caption2.semibold,
    color: Color = Palette.secondary,
) {
    AnimatedContent(
        targetState = value,
        modifier = modifier,
        transitionSpec = {
            if (targetState >= initialState) {
                (slideInVertically { h -> h } + fadeIn()).togetherWith(slideOutVertically { h -> -h } + fadeOut())
            } else {
                (slideInVertically { h -> -h } + fadeIn()).togetherWith(slideOutVertically { h -> h } + fadeOut())
            }
        },
        label = "SpinningCounter",
    ) { target ->
        Text(text = target.toString(), style = style, color = color)
    }
}

/**
 * Idea 30: Horizontal spring error shake modifier for rejected cards and invalid inputs.
 */
@Composable
fun Modifier.errorShake(trigger: Int): Modifier {
    if (trigger <= 0 || !ValueAnimator.areAnimatorsEnabled()) return this
    val offsetX = remember { Animatable(0f) }
    LaunchedEffect(trigger) {
        offsetX.animateTo(
            targetValue = 0f,
            animationSpec = keyframes {
                durationMillis = 420
                -8f at 60
                8f at 120
                -12f at 180
                12f at 240
                -6f at 300
                4f at 360
                0f at 420
            },
        )
    }
    return this.offset { IntOffset(offsetX.value.roundToInt(), 0) }
}

