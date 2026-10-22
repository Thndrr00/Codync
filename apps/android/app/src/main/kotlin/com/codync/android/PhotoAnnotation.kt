package com.codync.android

import android.content.Context
import android.graphics.Bitmap
import android.graphics.ImageDecoder
import android.graphics.Paint
import android.net.Uri
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.core.content.FileProvider
import com.codync.android.design.*
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.File
import java.io.FileOutputStream
import kotlin.math.min
import kotlin.math.roundToInt

/**
 * The photo annotation editor (idea 29): draw strokes over a photo just taken, undo the last
 * stroke, and send a copy (a new JPEG) with the strokes burned in. The bitmap is bounded on the
 * way in so an enormous photo can't exhaust memory. Closing without sending discards the temp file.
 */
@Composable fun PhotoAnnotation(source: Uri, sourceFile: File, onSend: (Uri) -> Unit) {
    val context = LocalContext.current
    var bitmap by remember { mutableStateOf<Bitmap?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var strokes by remember { mutableStateOf<List<List<Offset>>>(emptyList()) }
    var current by remember { mutableStateOf<List<Offset>>(emptyList()) }
    var canvasSize by remember { mutableStateOf(IntSize.Zero) }
    var exporting by remember { mutableStateOf(false) }
    val accent = Palette.accent
    val scope = rememberCoroutineScope()

    LaunchedEffect(source) {
        val decoded = withContext(Dispatchers.IO) { runCatching { decodeBounded(context, source) }.getOrNull() }
        bitmap = decoded
        if (decoded == null) error = "Couldn't open this photo."
    }
    // Leaving without sending discards the camera's temp file; sending already deleted it.
    DisposableEffect(sourceFile) { onDispose { runCatching { sourceFile.delete() } } }

    fun send() {
        val bmp = bitmap ?: return
        val size = canvasSize
        if (size.width <= 0 || size.height <= 0) return
        exporting = true
        scope.launch {
            val exported = withContext(Dispatchers.IO) { export(context, bmp, strokes, size, accent.toArgb()) }
            if (exported != null) { sourceFile.delete(); onSend(exported) } else { exporting = false; error = "Couldn't save the photo." }
        }
    }

    Column(Modifier.fillMaxSize()) {
        ModalHeader("Annotate") {
            IconButton("Undo", "arrow.counterclockwise", enabled = strokes.isNotEmpty()) { strokes = strokes.dropLast(1) }
            if (exporting) Box(Modifier.size(36.dp), contentAlignment = Alignment.Center) { Spinner() }
            else IconButton("Send", "checkmark", enabled = bitmap != null && !exporting) { send() }
        }
        Box(Modifier.weight(1f).fillMaxWidth().padding(20.dp), contentAlignment = Alignment.Center) {
            val bmp = bitmap
            when {
                error != null -> Text(error.orEmpty(), style = Type.body, color = Palette.danger)
                bmp == null -> Spinner(20.dp)
                else -> Canvas(Modifier.fillMaxSize().onSizeChanged { canvasSize = it }
                    .pointerInput(Unit) {
                        detectDragGestures(
                            onDragStart = { current = listOf(it) },
                            onDrag = { change, _ -> current = current + change.position },
                            onDragEnd = { if (current.isNotEmpty()) { strokes = strokes + current; current = emptyList() } },
                            onDragCancel = { current = emptyList() },
                        )
                    }) {
                    val scale = min(size.width / bmp.width, size.height / bmp.height)
                    val dw = bmp.width * scale
                    val dh = bmp.height * scale
                    val left = (size.width - dw) / 2
                    val top = (size.height - dh) / 2
                    drawImage(bmp.asImageBitmap(), dstOffset = IntOffset(left.roundToInt(), top.roundToInt()),
                        dstSize = IntSize(dw.roundToInt(), dh.roundToInt()))
                    for (stroke in strokes + listOf(current)) drawStroke(this, stroke, accent)
                }
            }
        }
    }
}

/** One polyline in the accent ink: a round 6 px stroke, or a dot for a tap. */
private fun drawStroke(draw: DrawScope, points: List<Offset>, ink: Color) {
    when {
        points.isEmpty() -> {}
        points.size == 1 -> draw.drawCircle(ink, radius = 3f, center = points.first())
        else -> {
            val path = Path().apply { moveTo(points.first().x, points.first().y); for (p in points.drop(1)) lineTo(p.x, p.y) }
            draw.drawPath(path, ink, style = Stroke(width = 6f, cap = StrokeCap.Round, join = StrokeJoin.Round))
        }
    }
}

/** Decodes with EXIF orientation applied and bounded to 2048 px on the longest side. */
private fun decodeBounded(context: Context, uri: Uri, maxDim: Int = 2048): Bitmap =
    ImageDecoder.decodeBitmap(ImageDecoder.createSource(context.contentResolver, uri)) { decoder, info, _ ->
        decoder.allocator = ImageDecoder.ALLOCATOR_SOFTWARE
        val longest = maxOf(info.size.width, info.size.height)
        if (longest > maxDim) decoder.setTargetSize(info.size.width * maxDim / longest, info.size.height * maxDim / longest)
    }

/** Renders the photo and strokes to a new JPEG at the image's displayed size (no letterbox bars). */
private fun export(context: Context, bmp: Bitmap, strokes: List<List<Offset>>, canvasSize: IntSize, accentArgb: Int): Uri? {
    val scale = min(canvasSize.width.toFloat() / bmp.width, canvasSize.height.toFloat() / bmp.height)
    val dw = (bmp.width * scale).roundToInt()
    val dh = (bmp.height * scale).roundToInt()
    val left = ((canvasSize.width - dw) / 2).toFloat()
    val top = ((canvasSize.height - dh) / 2).toFloat()
    if (dw <= 0 || dh <= 0) return null
    val out = Bitmap.createBitmap(dw, dh, Bitmap.Config.ARGB_8888)
    val canvas = android.graphics.Canvas(out)
    val paint = Paint(Paint.ANTI_ALIAS_FLAG)
    canvas.drawBitmap(bmp, null, android.graphics.Rect(0, 0, dw, dh), paint)
    val strokePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = accentArgb
        style = Paint.Style.STROKE
        strokeWidth = 6f
        strokeCap = Paint.Cap.ROUND
        strokeJoin = Paint.Join.ROUND
    }
    for (points in strokes) {
        if (points.size == 1) {
            val dot = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = accentArgb; style = Paint.Style.FILL }
            canvas.drawCircle(points[0].x - left, points[0].y - top, 3f, dot)
        } else if (points.size > 1) {
            val path = android.graphics.Path().apply {
                moveTo(points[0].x - left, points[0].y - top)
                for (p in points.drop(1)) lineTo(p.x - left, p.y - top)
            }
            canvas.drawPath(path, strokePaint)
        }
    }
    val dir = File(context.cacheDir, "shared-attachments").apply { mkdirs() }
    val file = File(dir, "annotated-${System.currentTimeMillis()}.jpg")
    FileOutputStream(file).use { stream -> out.compress(Bitmap.CompressFormat.JPEG, 90, stream) }
    out.recycle()
    return FileProvider.getUriForFile(context, context.packageName + ".files", file)
}
