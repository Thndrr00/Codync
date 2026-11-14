package com.codync.android

import android.webkit.MimeTypeMap
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.codync.android.core.OutgoingFile
import com.codync.android.design.*
import com.kyant.shapes.Capsule
import com.kyant.shapes.RoundedRectangle
import java.io.File
import java.io.IOException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

private val THUMBNAIL = 112.dp

/** A file going out with the next message, removable before it's sent (iOS `ComposerAttachment`): an image as a thumbnail, any other file as a chip. Its upload starts when it is attached: a ring shows the bytes the computer has and cancels it, and a failed file stays with its reason and a Retry. */
@Composable internal fun ComposerAttachment(file: OutgoingFile, upload: FileUpload?, cancel: () -> Unit, retry: () -> Unit, remove: () -> Unit) {
    val expectsImage = remember(file.name) {
        MimeTypeMap.getSingleton().getMimeTypeFromExtension(file.name.substringAfterLast('.', "").lowercase())?.startsWith("image/") == true
    }
    val pixels = with(LocalDensity.current) { THUMBNAIL.roundToPx() }
    var image by remember(file.path) { mutableStateOf<ImageBitmap?>(null) }
    var loaded by remember(file.path) { mutableStateOf(false) }
    LaunchedEffect(file.path, pixels) {
        image = thumbnail(file, pixels)
        loaded = true
    }
    val shown = image
    val uploading = upload?.takeIf { it.isUploading }
    val failed = upload?.takeIf { it.hasFailed }
    Column(verticalArrangement = Arrangement.spacedBy(3.dp)) {
        if (shown != null || expectsImage && !loaded) Box(Modifier.size(THUMBNAIL)) {
            Box(Modifier.fillMaxSize().clip(RoundedRectangle(16.dp)).background(Palette.surface)
                .semantics { contentDescription = "Image attachment: ${file.name}" }, contentAlignment = Alignment.Center) {
                if (shown != null) Image(shown, null, Modifier.fillMaxSize(), contentScale = ContentScale.Fit)
                else Symbol("photo", size = 20.dp, tint = Palette.secondary)
            }
            RemoveButton(file.name, Color.White, Modifier.align(Alignment.TopEnd).padding(4.dp).clip(Capsule()).background(Color.Black.copy(alpha = 0.75f)), remove)
            if (uploading != null) UploadRing(uploading, Modifier.align(Alignment.BottomStart).padding(6.dp), cancel)
        } else Row(Modifier.clip(Capsule()).background(Palette.surface).padding(start = 10.dp, end = 4.dp, top = 4.dp, bottom = 4.dp),
            verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            Symbol(AttachmentIcon.symbol(file.name), size = 18.dp, tint = Palette.secondary)
            Text(file.name, Modifier.widthIn(max = 160.dp), style = Type.footnote, color = Palette.text, maxLines = 1, overflow = TextOverflow.MiddleEllipsis)
            if (uploading != null) {
                Text("${uploading.percent}%", style = Type.caption2, color = Palette.secondary)
                UploadRing(uploading, Modifier, cancel)
            }
            RemoveButton(file.name, Palette.secondary, action = remove)
        }
        if (failed != null) Row(Modifier.widthIn(max = 180.dp), verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            Text(tr(failed.reason), Modifier.weight(1f, fill = false), style = Type.caption2, color = Palette.danger,
                maxLines = 1, overflow = TextOverflow.Ellipsis)
            val (source, pressed) = rememberPress()
            Text(tr("Retry"), Modifier.pressScale(pressed).clickable(source, null, role = Role.Button, onClick = retry)
                .semantics { contentDescription = "Retry upload" }, style = Type.caption2.bold, color = Palette.accent)
        }
    }
}

/** The bytes the computer has of one file, around a stop mark: a tap stops that file's upload. */
@Composable private fun UploadRing(upload: FileUpload, modifier: Modifier, cancel: () -> Unit) {
    val (source, pressed) = rememberPress()
    Box(modifier.size(20.dp).pressScale(pressed).clickable(source, null, role = Role.Button, onClick = cancel)
        .semantics { contentDescription = "Cancel upload" }, contentAlignment = Alignment.Center) {
        Canvas(Modifier.fillMaxSize()) {
            val stroke = 2.5.dp.toPx()
            drawArc(Color.White.copy(alpha = 0.3f), -90f, 360f, false, style = Stroke(stroke))
            drawArc(Palette.accent, -90f, 360f * upload.percent / 100f, false, style = Stroke(stroke, cap = StrokeCap.Round))
        }
        Symbol("xmark", size = 9.dp, tint = Color.White)
    }
}

@Composable private fun RemoveButton(name: String, tint: Color, modifier: Modifier = Modifier, action: () -> Unit) {
    val (source, pressed) = rememberPress()
    Box(modifier.size(28.dp).pressScale(pressed).clickable(source, null, role = Role.Button, onClick = action)
        .semantics { contentDescription = "Remove $name" }, contentAlignment = Alignment.Center) {
        Symbol("xmark", size = 15.dp, tint = tint)
    }
}

/** The file as a downsampled picture, off the main thread; null when it isn't one Android can decode. */
private suspend fun thumbnail(file: OutgoingFile, pixels: Int): ImageBitmap? = withContext(Dispatchers.IO) {
    // Not a picture (or one Android can't read): it shows as a file chip instead.
    try { AttachmentFiles.thumbnail(File(file.path), pixels).asImageBitmap() } catch (_: IOException) { null }
}
