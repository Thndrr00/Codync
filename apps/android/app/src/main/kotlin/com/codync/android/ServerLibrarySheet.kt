package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
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
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.codync.android.core.Attachment
import com.codync.android.core.LibraryItem
import com.codync.android.core.LinkState
import com.codync.android.core.ServerLibraryFilter
import com.codync.android.core.ServerLibraryItem
import com.codync.android.core.SharedFile
import com.codync.android.core.hasServerLibrary
import com.codync.android.design.*
import com.kyant.shapes.RoundedRectangle

/**
 * The server's Library (`docs/features/library.md`): every bot's artifacts and files of this computer, newest first by day.
 * Each row opens as it does in a chat; the filter only hides rows, the host already sent them newest first.
 */
@Composable internal fun ServerLibrarySheet(state: AppState, store: AppStore) {
    val online = state.actualConnection is LinkState.Ready
    var filter by remember { mutableStateOf(ServerLibraryFilter.All) }
    var image by remember { mutableStateOf<ServerImage?>(null) }
    var artifact by remember { mutableStateOf<OpenArtifact?>(null) }
    AnalyticsOnAppear(AnalyticsEvent.ScreenViewed(AnalyticsScreen.Library))
    LaunchedEffect(online) { if (online) store.loadServerLibrary() }
    val supported = hasServerLibrary(state.hello?.text("version"))
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader(tr("Server Library")) {
            IconButton("Refresh", "arrow.clockwise", enabled = online && supported) { store.loadServerLibrary() }
        }
        Column(
            Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(horizontal = 20.dp, vertical = 8.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            TabChoice(filter, ServerLibraryFilter.entries.map { it to it.title }, { filter = it })
            val items = state.serverLibrary
            when {
                !supported -> ServerLibraryMessage("Update this computer's Codync to see its Library.")
                items == null && "server-library" in state.busy ->
                    Box(Modifier.fillMaxWidth().padding(top = 40.dp), contentAlignment = Alignment.Center) { Spinner(20.dp) }
                items == null -> EmptyServerLibrary()
                else -> {
                    val shown = filter.apply(items)
                    if (shown.isEmpty()) EmptyServerLibrary()
                    else for ((period, members) in ServerLibraryItem.sections(shown)) {
                        Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                            SectionHeading(period.title, Modifier.padding(start = 2.dp))
                            Column(Modifier.fillMaxWidth().cardGroup()) {
                                for ((index, item) in members.withIndex()) {
                                    if (index > 0) Hairline(Modifier.padding(horizontal = 16.dp))
                                    ServerLibraryRow(item, state, store, { artifact = it }, { image = it })
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    when (val shown = image) {
        is ServerImage.Shared -> SharedImageSheet(shown.file, shown.entryId, state, store) { image = null }
        is ServerImage.Sent -> AttachmentImageSheet(shown.file, shown.botId, state, store) { image = null }
        null -> {}
    }
    ArtifactLayer(artifact) { open ->
        state.bots.firstOrNull { it.id == open.botId }?.let { owner ->
            ArtifactViewer(open, owner, state, store) { artifact = null }
        }
    }
}

/** A picture to show full size: one a bot shared, or one the user sent. */
private sealed interface ServerImage {
    data class Shared(val file: SharedFile, val entryId: String) : ServerImage
    data class Sent(val file: Attachment, val botId: String) : ServerImage
}

/** One row: its picture (a symbol or a thumbnail), its name, and whose bot published or shared it. */
@Composable private fun ServerLibraryRow(
    item: ServerLibraryItem,
    state: AppState,
    store: AppStore,
    openArtifact: (OpenArtifact) -> Unit,
    openImage: (ServerImage) -> Unit,
) {
    val from = tr("From {name}").replace("{name}", item.botName)
    when (val kind = item.item) {
        is LibraryItem.ArtifactItem -> LibraryRow(Modifier.plainButton {
            openArtifact(OpenArtifact(item.botId, kind.artifact.id, kind.artifact.title, kind.artifact.entryId))
        }, { Symbol("chart.bar.doc.horizontal", size = 22.dp, tint = Palette.secondary) }) {
            NameAndDetail(kind.artifact.title, from)
        }
        is LibraryItem.FileItem -> if (item.image) {
            LibraryRow(Modifier.plainButton { openImage(ServerImage.Shared(kind.file, kind.entryId)) }, {
                SharedImage(kind.file, kind.entryId, 144, state, store, Modifier.size(36.dp).clip(RoundedRectangle(8.dp)), ContentScale.Crop)
            }) { NameAndDetail(kind.file.name, from) }
        } else {
            val download = state.fileDownloads[kind.file.id]
            val busy = download is FileDownloadState.Downloading
            LibraryRow(Modifier.plainButton {
                if (busy) store.cancelFile(kind.file.id) else store.saveFile(kind.entryId, item.botId, kind.file)
            }, { Symbol(AttachmentIcon.symbol(kind.file.name), size = 22.dp, tint = Palette.secondary) }) {
                NameAndDetail(kind.file.name, "$from · ${detail(kind.file, download)}")
            }
        }
        is LibraryItem.AttachmentItem -> if (item.image) {
            LibraryRow(Modifier.plainButton { openImage(ServerImage.Sent(kind.file, item.botId)) }, {
                AttachmentImage(kind.file, item.botId, 144, state, store, Modifier.size(36.dp).clip(RoundedRectangle(8.dp)), ContentScale.Crop)
            }) { NameAndDetail(kind.file.name, from) }
        } else {
            LibraryRow(Modifier.semantics(mergeDescendants = true) {}, {
                Symbol(AttachmentIcon.symbol(kind.file.name), size = 22.dp, tint = Palette.secondary)
            }) { NameAndDetail(kind.file.name, "$from · ${formatBytes(kind.file.size)}") }
        }
    }
}

@Composable private fun EmptyServerLibrary() {
    Column(
        Modifier.fillMaxWidth().padding(horizontal = 32.dp, vertical = 48.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Symbol("books.vertical", size = 36.dp, tint = Palette.tertiary)
        Text(tr("Library is empty"), style = Type.headline, color = Palette.text, textAlign = TextAlign.Center)
        Text(
            tr("Artifacts and files from every bot on this computer show here."),
            style = Type.subheadline,
            color = Palette.secondary,
            textAlign = TextAlign.Center,
        )
    }
}

@Composable private fun ServerLibraryMessage(text: String) {
    Text(tr(text), Modifier.fillMaxWidth().padding(top = 24.dp), style = Type.subheadline, color = Palette.secondary)
}
