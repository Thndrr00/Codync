package com.codync.android

import android.graphics.Bitmap
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.provider.Settings
import android.util.Log
import android.view.FrameMetrics
import android.view.Window
import androidx.activity.compose.LocalActivity
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import com.codync.android.design.rememberConstellationPhase
import com.codync.android.design.reduceMotion
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import java.io.File
import java.util.Collections

/** Run separately from interaction checks so the measured frames contain only the 30-bot canvas. */
class ConstellationRenderingTest {
    @get:Rule val compose = createComposeRule()

    @Test fun recordThirtyBotFrames() {
        val frames = Collections.synchronizedList(mutableListOf<Long>())
        val bots = constellationFixtures()
        compose.mainClock.autoAdvance = false
        compose.setContent { Hosted {
            val activity = requireNotNull(LocalActivity.current)
            DisposableEffect(activity) {
                val listener = Window.OnFrameMetricsAvailableListener { _, metrics, _ ->
                    frames.add(metrics.getMetric(FrameMetrics.TOTAL_DURATION))
                }
                activity.window.addOnFrameMetricsAvailableListener(listener, Handler(Looper.getMainLooper()))
                onDispose { activity.window.removeOnFrameMetricsAvailableListener(listener) }
            }
            BotConstellation(bots, {}, { emptyList() }, Modifier.fillMaxSize())
        } }
        compose.waitForIdle()
        val end = SystemClock.uptimeMillis() + 5_000
        while (SystemClock.uptimeMillis() < end) {
            compose.mainClock.advanceTimeBy(32)
            SystemClock.sleep(32)
        }
        val timings = synchronized(frames) { frames.toList().sorted() }
        assertTrue("Expected rendered frames", timings.isNotEmpty())
        val report = "30 bots: frames=${timings.size}, p50=${timings[timings.size / 2] / 1e6}ms, " +
            "p95=${timings[(timings.size * 0.95).toInt().coerceAtMost(timings.lastIndex)] / 1e6}ms"
        Log.i("ConstellationPerformance", report)
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val directory = instrumentation.targetContext.getExternalFilesDir(null)!!
        File(directory, "constellation-frames.txt").writeText(report)
        requireNotNull(instrumentation.uiAutomation.takeScreenshot()).let { bitmap ->
            File(directory, "constellation-30.png").outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG, 100, it) }
            bitmap.recycle()
        }
    }

    @Test fun reducedMotionHasNoContinuousClock() {
        compose.mainClock.autoAdvance = false
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val resolver = instrumentation.targetContext.contentResolver
        val setting = Settings.Global.ANIMATOR_DURATION_SCALE
        val original = Settings.Global.getString(resolver, setting)
        fun shell(command: String) {
            instrumentation.uiAutomation.executeShellCommand(command).use {
                java.io.FileInputStream(it.fileDescriptor).use { stream -> stream.readBytes() }
            }
        }
        lateinit var phase: State<Float>
        var reduced = false
        try {
            shell("settings put global $setting 0")
            compose.setContent { Hosted {
                reduced = reduceMotion()
                phase = rememberConstellationPhase(!reduced)
            } }
            compose.waitForIdle()
            assertTrue(reduced)
            val before = phase.value
            compose.mainClock.advanceTimeBy(1_000)
            compose.runOnIdle { assertEquals(before, phase.value, 0f) }
            compose.mainClock.autoAdvance = true
            shell("settings put global $setting 1")
            compose.waitUntil(5_000) { !reduced }
            compose.runOnIdle { assertFalse(reduced) }
        } finally {
            shell(if (original == null) "settings delete global $setting" else "settings put global $setting $original")
        }
    }
}
