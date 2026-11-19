package com.codync.android

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.codync.android.core.LinkState
import com.codync.android.design.*
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.jsonPrimitive

/**
 * A bot's documentation search (the `codync-docs` server): which folder the computer indexes, what it holds, and the
 * choice between a folder of its own and the bot's working folder. The host builds and caches the index.
 */
@Composable internal fun DocsSettingsCard(botId: String, state: AppState, store: AppStore, workingFolder: String) {
    val scope = rememberCoroutineScope()
    val model = remember(botId) { DocsIndexModel(botId, store, scope) }
    val online = state.actualConnection is LinkState.Ready
    var choosing by remember { mutableStateOf(false) }
    LaunchedEffect(botId, online) { if (online) model.load() }
    CardSection(tr("Documentation search")) {
        row {
            Text(tr("Your bot searches this folder for documentation and code and cites the file and line. Until you choose another folder, it is the bot's working folder. Generated folders and what .gitignore lists are skipped."),
                style = Type.footnote, color = Palette.secondary)
        }
        val status = model.status
        if (status != null) {
            // The path is long and worth keeping: on its own line and selectable, not squeezed into a row's value.
            row {
                Column(verticalArrangement = Arrangement.spacedBy(3.dp)) {
                    Text(tr("Folder"), style = Type.body, color = Palette.text)
                    SelectionContainer { Text(status.folder, style = Type.caption, color = Palette.secondary) }
                }
            }
            row { ValueRow(tr("Files"), status.files.toString()) }
            row { ValueRow(tr("Size"), formatBytes(status.bytes)) }
            if (status.truncated) row {
                Text(tr("The index stopped at its limit of 4,000 files or 24 MB, so a search may miss some files."), style = Type.caption, color = Palette.warning)
            }
        } else if (online && model.error == null) row {
            Text(tr("Reading the index…"), style = Type.body, color = Palette.secondary)
        }
        model.error?.let { message -> row { Text(message, style = Type.body, color = Palette.danger) } }
        if (!online) row {
            Text(tr("Reconnect to this computer to change the indexed folder."), style = Type.caption, color = Palette.warning)
        }
        row {
            Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    SecondaryButton(tr("Choose folder"), enabled = online && !model.busy) { choosing = true }
                    SecondaryButton(tr("Use working folder"), enabled = online && !model.busy) { model.choose("") }
                }
                if (model.busy) {
                    Text(tr("Reading the index…"), style = Type.caption, color = Palette.secondary)
                }
            }
        }
    }
    CodyncSheet(choosing, { choosing = false }) {
        val start = model.status?.folder ?: workingFolder.ifEmpty { state.hello?.get("home")?.jsonPrimitive?.contentOrNull }
        FolderPicker(start, online, store::listDirs) { path -> model.choose(path); choosing = false }
    }
}
