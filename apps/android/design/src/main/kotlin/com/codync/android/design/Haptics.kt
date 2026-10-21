package com.codync.android.design

import android.os.Build
import android.view.HapticFeedbackConstants
import android.view.View
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.platform.LocalView

/** Semantic haptic feedback events emitted only on confirmed user interactions. */
enum class CodyncHaptic {
    TabSwitch,
    DragStart,
    /** Crossing a row boundary and swapping places during a reorder drag (never per pixel). */
    DragStep,
    DragDrop,
    MessageSent,
    Reaction,
    PermissionAction,
    Toggle,
}

/** Dispatches device haptic feedback when enabled by both app preferences and system settings. */
fun interface CodyncHaptics {
    fun perform(event: CodyncHaptic)

    companion object {
        val NoOp = CodyncHaptics {}
    }
}

val LocalCodyncHaptics = staticCompositionLocalOf { CodyncHaptics.NoOp }

/** Maps semantic events to Android [HapticFeedbackConstants] compatible with the running API level. */
internal fun hapticConstantFor(event: CodyncHaptic, sdkInt: Int = Build.VERSION.SDK_INT): Int = when (event) {
    CodyncHaptic.TabSwitch -> if (sdkInt >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
        HapticFeedbackConstants.SEGMENT_TICK
    } else {
        HapticFeedbackConstants.CLOCK_TICK
    }
    CodyncHaptic.DragStart -> if (sdkInt >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
        HapticFeedbackConstants.DRAG_START
    } else {
        HapticFeedbackConstants.LONG_PRESS
    }
    CodyncHaptic.DragStep -> if (sdkInt >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
        HapticFeedbackConstants.SEGMENT_FREQUENT_TICK
    } else {
        HapticFeedbackConstants.CLOCK_TICK
    }
    CodyncHaptic.DragDrop -> if (sdkInt >= Build.VERSION_CODES.R) {
        HapticFeedbackConstants.GESTURE_END
    } else {
        HapticFeedbackConstants.CONTEXT_CLICK
    }
    CodyncHaptic.MessageSent, CodyncHaptic.PermissionAction -> if (sdkInt >= Build.VERSION_CODES.R) {
        HapticFeedbackConstants.CONFIRM
    } else {
        HapticFeedbackConstants.KEYBOARD_TAP
    }
    CodyncHaptic.Reaction, CodyncHaptic.Toggle -> HapticFeedbackConstants.CLOCK_TICK
}

/** View-backed haptic dispatcher that respects system haptic settings (no override flags). */
class ViewCodyncHaptics(private val view: View, private val enabled: () -> Boolean) : CodyncHaptics {
    override fun perform(event: CodyncHaptic) {
        if (!enabled()) return
        view.performHapticFeedback(hapticConstantFor(event))
    }
}

/** In-memory recorder for unit and UI tests verifying that actions emit at most one pertinent feedback. */
class RecordingCodyncHaptics(var enabled: Boolean = true) : CodyncHaptics {
    private val recorded = mutableListOf<CodyncHaptic>()
    val events: List<CodyncHaptic> get() = recorded.toList()

    override fun perform(event: CodyncHaptic) {
        if (!enabled) return
        recorded += event
    }

    fun clear() {
        recorded.clear()
    }
}

@Composable fun rememberCodyncHaptics(enabled: Boolean): CodyncHaptics {
    val view = LocalView.current
    return remember(view, enabled) { ViewCodyncHaptics(view) { enabled } }
}
