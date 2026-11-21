package com.codync.android

import android.net.Uri
import android.widget.MediaController
import android.widget.VideoView
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.snap
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.PointerInputChange
import androidx.compose.ui.input.pointer.awaitPointerEvent
import androidx.compose.ui.input.pointer.awaitPointerEventScope
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import com.codync.android.design.*
import java.io.File
import kotlinx.coroutines.CancellationException

/**
 * Pictures and clips full size, paged sideways while `items` is set (iOS image viewer). The header
 * names the page and counts it; [trailing] adds the page's own actions.
 */
@Composable internal fun <T : Any> MediaViewerSheet(items: List<T>?, start: Int, name: (T) -> String, dismiss: () -> Unit,
    trailing: (@Composable (T) -> Unit)? = null, content: @Composable (T, Modifier, Boolean) -> Unit) {
    if (items == null) return
    CodyncSheet(true, dismiss) {
        val pager = rememberPagerState(initialPage = start.coerceIn(0, items.lastIndex)) { items.size }
        val reduced = reduceMotion()
        // Fit is 1x. Pinch or double-tap zooms up to 4x, then a drag pans; Fit returns to the screen.
        var desired by remember { mutableFloatStateOf(1f) }
        var pan by remember { mutableStateOf(Offset.Zero) }
        val scale by animateFloatAsState(desired, if (reduced) snap() else Motion.layout(), label = "Media zoom")
        val current by rememberUpdatedState(scale)
        LaunchedEffect(pager.currentPage) { desired = 1f; pan = Offset.Zero }
        fun fit() { desired = 1f; pan = Offset.Zero }
        Column(Modifier.fillMaxSize().background(Palette.background)) {
            ModalHeader(name(items[pager.currentPage])) {
                Text("${pager.currentPage + 1} of ${items.size}", Modifier.padding(end = 8.dp), style = Type.caption, color = Palette.tertiary)
                SecondaryButton(if (scale > 1.01f) "Fit" else "Zoom") {
                    if (scale > 1.01f) fit() else { desired = 2.5f; pan = Offset.Zero }
                }
                trailing?.invoke(items[pager.currentPage])
            }
            HorizontalPager(pager, Modifier.fillMaxSize(), contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 12.dp),
                pageSpacing = 12.dp) { page ->
                val active = page == pager.currentPage
                Box(
                    Modifier.fillMaxSize().padding(vertical = 12.dp)
                        .pointerInput(Unit) { zoomable(current, { desired = it }, { pan += it }) { pan = Offset.Zero } }
                        .pointerInput(Unit) {
                            detectTapGestures(onDoubleTap = {
                                if (current.value > 1.01f) fit() else { desired = 2.5f; pan = Offset.Zero }
                            })
                        }
                        .graphicsLayer(scaleX = scale, scaleY = scale, translationX = pan.x, translationY = pan.y),
                    contentAlignment = Alignment.Center,
                ) {
                    content(items[page], Modifier.fillMaxSize(), active)
                }
            }
        }
    }
}

/** Pinch (two pointers) and pan (one pointer, only once zoomed). Single drags at fit are left to
 * the pager; only consumed pointers stop it from turning the page. */
private suspend fun androidx.compose.ui.input.pointer.PointerInputScope.zoomable(
    current: androidx.compose.runtime.State<Float>,
    zoom: (Float) -> Unit,
    panBy: (Offset) -> Unit,
    resetPan: () -> Unit,
) = awaitPointerEventScope {
    while (true) {
        var event = awaitPointerEvent(PointerEventPass.Initial)
        var pressed = event.changes.filter { it.pressed }
        if (pressed.isEmpty()) continue
        if (pressed.size >= 2) {
            var distance = pressed.distance()
            var center = pressed.center()
            while (true) {
                event = awaitPointerEvent(PointerEventPass.Initial)
                pressed = event.changes.filter { it.pressed }
                if (pressed.size < 2) break
                val nextDistance = pressed.distance()
                val nextCenter = pressed.center()
                if (distance > 0f) zoom((current.value * nextDistance / distance).coerceIn(1f, 4f))
                if (current.value > 1.01f) panBy(nextCenter - center) else resetPan()
                pressed.forEach { it.consume() }
                distance = nextDistance
                center = nextCenter
            }
        } else if (current.value > 1.01f) {
            var last = pressed.first().position
            while (true) {
                event = awaitPointerEvent(PointerEventPass.Initial)
                pressed = event.changes.filter { it.pressed }
                if (pressed.size >= 2) break
                val pointer = pressed.firstOrNull() ?: break
                panBy(pointer.position - last)
                pointer.consume()
                last = pointer.position
            }
        }
    }
}

private fun List<PointerInputChange>.distance(): Float =
    if (size < 2) 0f else (this[0].position - this[1].position).getDistance()

private fun List<PointerInputChange>.center(): Offset =
    if (isEmpty()) Offset.Zero else fold(Offset.Zero) { total, change -> total + change.position } / size.toFloat()

/** A clip played by the system player while its page is the one showing. */
@Composable internal fun VideoPlayer(load: suspend () -> File, active: Boolean, modifier: Modifier) {
    var file by remember { mutableStateOf<File?>(null) }
    var view by remember { mutableStateOf<VideoView?>(null) }
    LaunchedEffect(Unit) {
        try { file = load() } catch (cancelled: CancellationException) { throw cancelled } catch (_: Exception) { /* A still frame stays. */ }
    }
    val ready = file
    if (ready == null) {
        Box(modifier.background(Color.Black).semantics { contentDescription = "Loading video" }, contentAlignment = Alignment.Center) { Spinner() }
        return
    }
    AndroidView(factory = { context ->
        VideoView(context).apply {
            MediaController(context).also { controller -> controller.setAnchorView(this); setMediaController(controller) }
            setVideoURI(Uri.fromFile(ready))
            setOnPreparedListener { if (active) start() }
            view = this
        }
    }, update = { player ->
        if (active) { if (!player.isPlaying) player.start() } else if (player.isPlaying) player.pause()
    }, modifier = modifier.background(Color.Black))
    DisposableEffect(Unit) { onDispose { view?.stopPlayback() } }
}
