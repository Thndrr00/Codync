package com.codync.android

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
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
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.unit.dp
import com.codync.android.design.*
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch

/**
 * The board, the saved teams and the read-only links this computer keeps (iOS `WorkspaceSheet`).
 * Every change is a host call, so every client sees the same board.
 */
@Composable
internal fun WorkspaceSheet(store: AppStore) {
    var tab by remember { mutableStateOf(WorkspaceTab.Board) }
    var tasks by remember { mutableStateOf<List<BoardTask>>(emptyList()) }
    var templates by remember { mutableStateOf<List<TeamTemplate>>(emptyList()) }
    var links by remember { mutableStateOf<List<SharedLink>>(emptyList()) }
    var error by remember { mutableStateOf<String?>(null) }
    var loading by remember { mutableStateOf(true) }
    val scope = rememberCoroutineScope()

    suspend fun load() {
        loading = true
        try {
            tasks = store.boardTasks()
            templates = store.teamTemplates()
            links = store.sharedLinks()
            error = null
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (failure: Exception) {
            error = failure.message
        } finally {
            loading = false
        }
    }

    fun run(block: suspend () -> Unit) {
        scope.launch {
            try {
                block()
                error = null
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (failure: Exception) {
                error = failure.message
            }
        }
    }

    LaunchedEffect(Unit) { load() }

    Column(Modifier.fillMaxWidth()) {
        ModalHeader("Workspace") { if (loading) Spinner() }
        CardForm {
            SegmentedChoice(tab, WorkspaceTab.entries.map { it to it.label }, select = { tab = it })
            when (tab) {
                WorkspaceTab.Board -> {
                    var title by remember { mutableStateOf("") }
                    var detail by remember { mutableStateOf("") }
                    var more by remember { mutableStateOf(false) }
                    CardSection("Add a card", "A card can name the bot that owns it; moving it is yours to do.") {
                        row {
                            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                                FieldBox(title, { title = it }, "What needs doing")
                                if (more) FieldBox(detail, { detail = it }, "Detail (optional)", minLines = 2, maxLines = 4)
                                Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                                    LinkButton(if (more) "Fewer options" else "More options") { more = !more }
                                    Box(Modifier.weight(1f))
                                    SecondaryButton("Add", enabled = title.isNotBlank()) {
                                        val text = title.trim()
                                        run {
                                            tasks = store.boardTasks()
                                            store.saveBoardTask(text, detail.trim())
                                            tasks = store.boardTasks()
                                            title = ""
                                            detail = ""
                                            more = false
                                        }
                                    }
                                }
                            }
                        }
                    }
                    for (status in BoardTask.statuses) {
                        val column = tasks.filter { it.status == status }
                        if (column.isNotEmpty()) CardSection(statusTitle(status)) {
                            for (task in column) row {
                                BoardTaskRow(task,
                                    onMove = { next -> run {
                                        store.moveBoardTask(task.id, next)
                                        tasks = store.boardTasks()
                                    } },
                                    onDelete = { run {
                                        store.deleteBoardTask(task.id)
                                        tasks = store.boardTasks()
                                    } })
                            }
                        }
                    }
                    if (tasks.isEmpty() && !loading) CardSection {
                        row { Text("Nothing on the board yet.", style = Type.body, color = Palette.secondary) }
                    }
                }
                WorkspaceTab.Teams -> {
                    if (templates.isEmpty() && !loading) CardSection {
                        row { Text("No saved teams yet. Save a group as a team on the computer.", style = Type.body, color = Palette.secondary) }
                    }
                    for (template in templates) CardSection(template.name, template.description.ifEmpty { null }) {
                        if (template.members.isNotEmpty()) row {
                            Text(template.members.joinToString(" · "), style = Type.footnote, color = Palette.secondary)
                        }
                        row {
                            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                                SecondaryButton("Add to this computer") { run { store.installTeamTemplate(template.id) } }
                            }
                        }
                    }
                }
                WorkspaceTab.Links -> {
                    val clipboard = LocalClipboardManager.current
                    val uri = LocalUriHandler.current
                    if (links.isEmpty() && !loading) CardSection {
                        row { Text("No read-only links. Publishing a document or artifact creates one.", style = Type.body, color = Palette.secondary) }
                    }
                    for (link in links) CardSection(link.title, "Anyone with the link can read it until it expires or is revoked.") {
                        row { Text(link.url, style = Type.footnote, color = Palette.secondary, maxLines = 1) }
                        row {
                            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                                SecondaryButton("Copy link") { clipboard.setText(AnnotatedString(link.url)) }
                                SecondaryButton("Open") { if (link.url.isNotEmpty()) uri.openUri(link.url) }
                                Box(Modifier.weight(1f))
                                SecondaryButton("Revoke") { run {
                                    store.revokeSharedLink(link.token)
                                    links = store.sharedLinks()
                                } }
                            }
                        }
                    }
                }
            }
            if (error != null) Text(error ?: "", style = Type.footnote, color = Palette.danger)
        }
    }
}

@Composable private fun BoardTaskRow(task: BoardTask, onMove: (String) -> Unit, onDelete: () -> Unit) {
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        Symbol(if (task.isDone) "checkmark.circle.fill" else "circle", size = 18.dp,
            tint = if (task.isDone) Palette.accent else Palette.tertiary)
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(task.title, style = Type.body, color = Palette.text)
            if (task.detail.isNotEmpty()) Text(task.detail, style = Type.footnote, color = Palette.secondary, maxLines = 2)
        }
        DropdownMenu({ BoardTask.statuses.filter { it != task.status }.map { status ->
            MenuItem("Move to ${statusTitle(status)}", icon = "arrow.right") { onMove(status) }
        } + MenuItem("Delete", icon = "trash", destructive = true, divider = true, action = onDelete) }) {
            Symbol("ellipsis", size = 16.dp, tint = Palette.secondary)
        }
    }
}

private enum class WorkspaceTab(val label: String) {
    Board("Board"),
    Teams("Teams"),
    Links("Links"),
}

private fun statusTitle(status: String): String = when (status) {
    "doing" -> "Doing"
    "waiting" -> "Waiting"
    "done" -> "Done"
    else -> "To do"
}

