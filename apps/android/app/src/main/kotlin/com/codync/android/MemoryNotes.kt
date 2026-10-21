package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.selection.SelectionContainer
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
import androidx.compose.ui.unit.dp
import com.codync.android.core.MemoryFile
import com.codync.android.core.MemoryFileDetail
import com.codync.android.core.MemoryFileTarget
import com.codync.android.core.MemoryFileVersion
import com.codync.android.core.MemoryFiles
import com.codync.android.design.*
import kotlinx.coroutines.launch

private const val NOTE_TEMPLATE = "---\ndescription: When to read this note\n---\n"
private val NOTE_NAME = Regex("^[a-z0-9-]+\\.md$")

/** One layer's long-form notes, under its entries: name, description, size and date, marked past a cap (iOS `MemoryFilesList`). */
@Composable internal fun MemoryNotes(memory: MemoryModel, files: MemoryFiles, shared: Boolean, state: AppState) {
    var open by remember { mutableStateOf<MemoryFileTarget?>(null) }
    val folder = if (shared) files.yours else files.bot
    val live = folder.files.filter { !it.deleted }
    val gone = folder.files.filter { it.deleted }
    Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("Notes", Modifier.weight(1f), style = Type.footnote.semibold, color = Palette.secondary)
            IconButton("New note", "doc.badge.plus", enabled = !memory.busy) { open = MemoryFileTarget(null, shared) }
        }
        if (live.isEmpty()) Text("Longer notes, one topic each. The bot reads one when its topic comes up.", style = Type.footnote, color = Palette.tertiary)
        else if (live.size > files.folderCap) Text("${live.size} notes, over the ${files.folderCap}-note cap. Merge some.", style = Type.footnote, color = Palette.attention)
        for (file in live + gone) NoteRow(file, files.cap) { open = MemoryFileTarget(file.name, shared) }
    }
    CodyncSheet(open, { open = null }) { MemoryNoteSheet(memory, it, state) }
}

@Composable private fun NoteRow(file: MemoryFile, cap: Int, open: () -> Unit) {
    Column(Modifier.fillMaxWidth().plainButton(action = open).padding(vertical = 4.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Symbol(if (file.deleted) "doc" else "doc.text", size = 16.dp, tint = if (file.deleted) Palette.tertiary else Palette.text)
            Text(file.name, style = Type.body, color = if (file.deleted) Palette.tertiary else Palette.text)
        }
        val description = file.description
        if (description != null && !file.deleted) Text(description, style = Type.footnote, color = Palette.secondary, maxLines = 2)
        val over = file.tokens > cap && !file.deleted
        Text(if (file.deleted) "Deleted ${ago(file.updatedAt)}" else "${file.tokens} tokens${if (over) ", over the $cap cap" else ""} · ${ago(file.updatedAt)}",
            style = Type.caption, color = if (over) Palette.attention else Palette.secondary)
    }
}

/** One note: read it, edit it, delete it, or bring back an earlier version (iOS `MemoryFileView`). */
@Composable private fun MemoryNoteSheet(memory: MemoryModel, target: MemoryFileTarget, state: AppState) {
    val dismiss = LocalDismissModal.current
    val scope = rememberCoroutineScope()
    var detail by remember(target) { mutableStateOf<MemoryFileDetail?>(null) }
    var editing by remember(target) { mutableStateOf(target.name == null) }
    var draft by remember(target) { mutableStateOf(if (target.name == null) NOTE_TEMPLATE else "") }
    var name by remember(target) { mutableStateOf("") }
    var confirmDelete by remember { mutableStateOf(false) }
    val validName = NOTE_NAME.matches(name)
    // An error from an earlier action belongs to that action, not to this note.
    LaunchedEffect(target) { memory.error = null; target.name?.let { detail = memory.file(it, target.shared) } }

    /** Saves (`content`), deletes (null) or restores `version`, over what was loaded. */
    fun change(content: String? = null, version: Long? = null) {
        val file = target.name ?: name
        val base = if (target.name == null) null else detail?.hash
        scope.launch {
            if (!memory.changeFile(file, target.shared, base, content, version)) return@launch
            if (content == null && version == null) { dismiss(); return@launch }
            editing = false
            if (target.name == null) { dismiss(); return@launch }
            detail = memory.file(file, target.shared)
        }
    }

    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader(target.name ?: "New note") {
            val content = detail?.content
            if (memory.busy) Spinner()
            else if (editing) IconButton("Save", "checkmark", enabled = draft.isNotBlank() && (target.name != null || validName)) { change(content = draft) }
            else if (content != null) {
                IconButton("Edit", "pencil") { draft = content; editing = true }
                IconButton("Delete", "trash") { confirmDelete = true }
            }
        }
        CardForm {
            memory.error?.let { CardSection { row { SelectionContainer { Text(it, style = Type.body, color = Palette.danger) } } } }
            val loaded = detail
            if (editing) {
                if (target.name == null) CardSection(footer = "Lowercase letters, digits and dashes, ending in .md.") {
                    row { FieldBox(name, { name = it }, "people.md", capitalize = false) }
                }
                CardSection(footer = "The description: line says when the bot should read it.") {
                    row { Column(Modifier.heightIn(min = 240.dp)) { FieldBox(draft, { draft = it }, "", 12, 40, style = Type.footnote.monospaced, capitalize = false) } }
                }
            } else if (loaded != null) {
                CardSection(footer = loaded.path) {
                    row {
                        if (loaded.content != null) SelectionContainer { Text(loaded.content!!, Modifier.fillMaxWidth(), style = Type.footnote.monospaced, color = Palette.text) }
                        else Text("Deleted. Bring back a version below.", style = Type.body, color = Palette.secondary)
                    }
                }
                CardSection("Versions") {
                    for (version in loaded.versions) row { VersionRow(version, loaded.content, state, !memory.busy) { change(version = version.id) } }
                }
            } else if (memory.error == null) CardSection { row { Text("Loading…", style = Type.body, color = Palette.secondary) } }
        }
    }
    CodyncDialog("Delete ${target.name ?: "this note"}?", confirmDelete, { confirmDelete = false }, message = "Its versions stay, so you can bring it back.") {
        listOf(DialogAction("Delete", destructive = true) { change(content = null) })
    }
}

@Composable private fun VersionRow(version: MemoryFileVersion, current: String?, state: AppState, enabled: Boolean, restore: () -> Unit) {
    val by = when (version.author) {
        "user" -> "You"
        "bot" -> state.bots.firstOrNull { it.id == version.authorBot }?.name ?: "A bot"
        "dream" -> "Dreaming"
        else -> "This computer"
    }
    val what = when (version.cause) {
        "delete" -> "$by deleted it"
        "restore" -> "$by brought back a version"
        "undo" -> "$by undid a change"
        else -> "$by changed it"
    }
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(what, style = Type.body, color = Palette.text)
            Text(ago(version.at), style = Type.caption, color = Palette.secondary)
        }
        if (version.content != null && version.content != current) IconButton("Bring back this version", "arrow.counterclockwise", enabled = enabled, action = restore)
    }
}

/** The info button beside the harness's own memory switch: what turning it on means (iOS `HarnessMemorySheet`). */
@Composable internal fun HarnessMemorySheet(harness: String, readable: Boolean) {
    val details = listOf(
        Triple("brain", "A memory of its own", "$harness keeps notes apart from Codync's memory. It's off by default, so this bot has one memory and you see and change all of it here."),
        Triple("square.on.square", "Remembered twice", "With both on, the bot may save the same thing twice, and the two can disagree after something changes."),
        Triple("trash", "Forgetting", "Forgetting something here doesn't remove it from $harness's notes. Codync can't change them."),
        Triple("eye", "Where to see it", if (readable) "Its notes show read-only under the $harness tab." else "Codync can't show it; look in $harness itself."),
    )
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader("$harness's own memory")
        CardForm {
            CardSection {
                for ((icon, title, detail) in details) row {
                    Row(verticalAlignment = Alignment.Top, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                        Symbol(icon, size = 18.dp, tint = Palette.secondary, modifier = Modifier.width(22.dp).padding(top = 2.dp))
                        Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                            Text(title, style = Type.body, color = Palette.text)
                            Text(detail, style = Type.footnote, color = Palette.secondary)
                        }
                    }
                }
            }
        }
    }
}
