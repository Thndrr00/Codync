package com.codync.android

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import com.codync.android.core.Brain
import com.codync.android.core.BrainBundle
import com.codync.android.core.WireJson
import com.codync.android.design.*
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * The brains a bot can work in (iOS `BrainSheet`): the Fleet one, which is the memory all bots used to share, and the
 * projects, to pick one, make one, rename it, delete it, or move a brain to another computer.
 */
@Composable internal fun BrainSheet(memory: MemoryModel) {
    val dismiss = LocalDismissModal.current
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var name by remember { mutableStateOf("") }
    var creating by remember { mutableStateOf(false) }
    var renaming by remember { mutableStateOf<Brain?>(null) }
    var deleting by remember { mutableStateOf<Brain?>(null) }
    var notice by remember { mutableStateOf<String?>(null) }
    val brains = memory.brains.orEmpty()
    val current = memory.view?.brain
    LaunchedEffect(Unit) { if (memory.brains == null) memory.loadBrains() }
    val import = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        if (uri == null) return@rememberLauncherForActivityResult
        scope.launch {
            val bytes = withContext(Dispatchers.IO) { context.contentResolver.openInputStream(uri)?.use { it.readBytes() } }
            val bundle = bytes?.let { runCatching { WireJson.decodeFromString<BrainBundle>(it.decodeToString()) }.getOrNull() }
            if (bundle == null) { notice = "That file isn't a Codync brain file."; return@launch }
            val skipped = memory.importBrain(bundle) ?: return@launch
            notice = if (skipped > 0) "Imported, $skipped files were turned away." else "Imported."
        }
    }
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader("Brain") { IconButton("Close", "xmark", action = dismiss) }
        CardForm {
            CardSection(footer = "A brain is memory bots share: a project's, or the Fleet one, which every bot that " +
                "uses the shared memory reads. A bot reads its brain's memory and no other project's.") {
                for (brain in brains) row {
                    BrainChoiceRow(brain, brain.id == current?.id) {
                        if (brain.id != current?.id) memory.joinBrain(brain)
                    }
                }
                if (current != null) row {
                    SecondaryButton("Leave this brain", enabled = !memory.busy) { memory.leaveBrain() }
                }
            }
            CardSection(title = "New brain") {
                row {
                    SecondaryButton("Create a brain", enabled = !memory.busy) { name = ""; creating = true }
                }
            }
            if (brains.any { !it.shared }) CardSection(title = "Projects",
                footer = "Deleting a brain keeps its bots and their own memory; what the brain held goes.") {
                for (brain in brains.filter { !it.shared }) row {
                    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text(brain.name, Modifier.weight(1f), style = Type.body, color = Palette.text)
                        IconButton("Rename ${brain.name}", "pencil", enabled = !memory.busy) { name = brain.name; renaming = brain }
                        IconButton("Save ${brain.name} to a file", "square.and.arrow.up", enabled = !memory.busy) { memory.exportBrain(brain) }
                        IconButton("Delete ${brain.name}", "trash", enabled = !memory.busy) { deleting = brain }
                    }
                }
            }
            CardSection(title = "Import a brain", footer = notice) {
                row {
                    SecondaryButton("Import from a file", enabled = !memory.busy) { import.launch(arrayOf("application/json")) }
                }
            }
            memory.error?.let { row { Text(it, style = Type.body, color = Palette.danger) } }
        }
    }
    CodyncDialog("New brain", creating, { creating = false }, "What the bots in it share.", field = {
        BrainNameField(name) { name = it }
    }) { listOf(DialogAction("Create") { if (name.isNotBlank()) memory.saveBrain(name.trim()) }) }
    CodyncDialog(renaming?.let { "Rename ${it.name}" } ?: "Rename", renaming != null, { renaming = null }, field = {
        BrainNameField(name) { name = it }
    }) {
        listOf(DialogAction("Rename") {
            val brain = renaming
            if (brain != null && name.isNotBlank()) memory.saveBrain(name.trim(), brain.id)
        })
    }
    CodyncDialog(deleting?.let { "Delete the brain ${it.name}?" } ?: "Delete", deleting != null, { deleting = null },
        "Its bots go back to the Fleet brain and keep their own memory. What the brain held is gone.") {
        listOf(DialogAction("Delete", destructive = true) { deleting?.let { memory.deleteBrain(it) } })
    }
}

/** A brain's name, focused as its dialog opens. */
@Composable private fun BrainNameField(name: String, change: (String) -> Unit) {
    val focus = remember { FocusRequester() }
    LaunchedEffect(Unit) { focus.requestFocus() }
    PlainTextField(name, change, Modifier.fillMaxWidth().focusRequester(focus).pill(Palette.background), "Brain name", singleLine = true)
}

/** One brain to pick, with a checkmark on the bot's own. */
@Composable private fun BrainChoiceRow(brain: Brain, selected: Boolean, pick: () -> Unit) {
    Row(Modifier.fillMaxWidth().clickable(enabled = !selected) { pick() }.padding(vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(if (brain.shared) "Fleet (every bot)" else brain.name, style = Type.body, color = Palette.text)
            Text(if (brain.shared) "What your bots share. Joining keeps the memory you already share."
                else "${brain.members.size} bots", style = Type.footnote, color = Palette.secondary)
        }
        if (selected) Symbol("checkmark", size = 12.dp, tint = Palette.text)
    }
}
