package com.codync.android

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import androidx.compose.animation.animateContentSize
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.codync.android.design.*
import com.codync.android.core.ComposioApp
import com.codync.android.core.MarketConnector
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.roundToInt
import kotlin.math.sin

private enum class DocumentsTab { Library, Reviews, Activity, Graph }

/** A bot's host-backed library and exact-draft review queue. */
@Composable fun DocumentsCard(botId: String, state: AppState, store: AppStore) {
    val model = remember(botId, state.generation) { store.documentsFor(botId) }
    var tab by remember(botId) { mutableStateOf(DocumentsTab.Library) }
    var conversation by remember(botId, state.generation) { mutableStateOf<String?>(null) }
    var trace by remember(botId, state.generation) { mutableStateOf(false) }
    var setupEntry by remember(botId, state.generation) { mutableStateOf<String?>(null) }
    var app by remember { mutableStateOf<ComposioApp?>(null) }
    var connector by remember { mutableStateOf<MarketConnector?>(null) }
    val connect = ConnectionRequestActions({ id, item -> setupEntry = id; app = item }, { id, item -> setupEntry = id; connector = item })
    val bot = state.bots.firstOrNull { it.id == botId }
    val context = LocalContext.current
    LaunchedEffect(model) { model.watch() }
    LaunchedEffect(tab) { if (tab == DocumentsTab.Graph && model.graphNodes.isEmpty() && !model.graphLoading) model.loadGraph() }
    LaunchedEffect(conversation, state.generation, state.connection::class) {
        conversation?.let { store.loadDocumentConversation(botId, it) }
    }
    CardSection("Documents", accessory = {
        IconButton("New document", "plus", enabled = !model.busy && !model.saving) { model.create() }
        IconButton("Refresh documents", "arrow.clockwise", enabled = !model.loading) { model.refresh() }
    }) {
        row { SegmentedChoice(tab, listOf(DocumentsTab.Library to "Library", DocumentsTab.Reviews to "Reviews", DocumentsTab.Activity to "Activity", DocumentsTab.Graph to "Graph"), { tab = it }) }
        model.error?.let { row { Text(it, style = Type.body, color = Palette.danger) } }
        if (model.loading) row { Text("Loading documents…", style = Type.body, color = Palette.secondary) }
        when (tab) {
            DocumentsTab.Library -> {
                row { FieldBox(model.query, model::search, "Search documents") }
                if (model.dirty) row { LinkButton("Continue unsaved draft", action = model::resume) }
                if (!model.loading && model.items.isEmpty()) row {
                    Text(if (model.query.isBlank()) "No documents yet. Create a page to keep work outside the conversation." else "No documents match this search.", style = Type.body, color = Palette.secondary)
                }
                for ((document, depth) in documentTree(model.items, model.query.isNotBlank())) row {
                    Row(Modifier.fillMaxWidth().padding(start = (depth.coerceAtMost(8) * 16).dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                        Column(Modifier.weight(1f).plainButton { model.open(document.id) }, verticalArrangement = Arrangement.spacedBy(4.dp)) {
                            Text(document.title, style = Type.body.semibold, color = Palette.text)
                            val parent = document.parentId?.let { id -> model.items.firstOrNull { it.id == id }?.title ?: "Subpage" }
                            Text(listOfNotNull(parent, "Revision ${document.revision}", ago(document.updatedAt)).joinToString(" · "), style = Type.caption, color = Palette.secondary)
                        }
                        IconButton("Add subpage to ${document.title}", "doc.badge.plus", enabled = !model.saving) { model.create(document.id) }
                        IconButton("Discuss ${document.title}", "bubble.left.and.bubble.right", enabled = !model.dirty && !model.saving && bot != null) {
                            conversation = "doc:${document.id}"
                        }
                    }
                }
                if (model.next != null) row { LinkButton("Load more documents", enabled = !model.loading) { model.more() } }
            }
            DocumentsTab.Reviews -> {
                if (model.reviews.isEmpty()) row { Text("No document reviews. Drafts proposed by this bot appear here before they are saved.", style = Type.body, color = Palette.secondary) }
                for (review in model.reviews) row {
                    Column(Modifier.fillMaxWidth().plainButton { model.inspectReview(review.id) }, verticalArrangement = Arrangement.spacedBy(4.dp)) {
                        Text(review.title, style = Type.body.semibold, color = Palette.text)
                        Text(review.status.replaceFirstChar { it.uppercase() } + " · " + ago(review.createdAt), style = Type.caption, color = Palette.secondary)
                    }
                }
            }
            DocumentsTab.Activity -> {
                row {
                    LinkButton("Copy activity CSV") {
                        model.export { csv ->
                            (context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager)
                                .setPrimaryClip(ClipData.newPlainText("Computer activity CSV", csv))
                        }
                    }
                }
                if (model.activity.isEmpty()) row { Text("No computer activity yet. Typed values, file contents and commands are excluded.", style = Type.body, color = Palette.secondary) }
                for (entry in model.activity) row {
                    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                        Text(entry.action + " · " + entry.result, style = Type.body, color = Palette.text)
                        Text("${entry.actorKind}: ${entry.actorId} · ${ago(entry.createdAt)}", style = Type.caption, color = Palette.secondary)
                    }
                }
            }
            DocumentsTab.Graph -> {
                row { LinkButton("Refresh graph", enabled = !model.graphLoading) { model.loadGraph() } }
                if (model.graphLoading) row { Text("Reading the graph…", style = Type.body, color = Palette.secondary) }
                if (!model.graphLoading && model.graphNodes.isEmpty()) row { Text("No pages yet.", style = Type.body, color = Palette.secondary) }
                if (model.graphNodes.isNotEmpty()) row { DocumentGraph(model.graphNodes, model.graphEdges, model::open) }
            }
        }
    }
    CodyncSheet(model.editor, model::closeEditor) { DocumentEditor(model) }
    CodyncSheet(model.reviewOpen, model::closeReview) { DocumentReviewSheet(model) }
    CodyncSheet(conversation, { conversation = null; trace = false }) { root ->
        if (bot != null) RepliesView(bot, root, state.copy(thread = root), store, connect,
            documentTitle = model.items.firstOrNull { it.id == root.removePrefix("doc:") }?.title
                ?: model.draft?.takeIf { it.id == root.removePrefix("doc:") }?.title) { trace = true }
    }
    CodyncSheet(trace, { trace = false }) { if (bot != null) TraceView(bot, conversation, state) }
    CodyncSheet(app, { app = null; setupEntry = null }) { ComposioConnectSheet(it, state, store, setupEntry) { store.refreshPlugins() } }
    CodyncSheet(connector, { connector = null; setupEntry = null }) { InstallConnectorSheet(it, state, store, setupEntry) }
}

@Composable private fun DocumentEditor(model: DocumentsModel) {
    var preview by remember { mutableStateOf(false) }
    var confirmDelete by remember { mutableStateOf(false) }
    var confirmDiscard by remember { mutableStateOf(false) }
    val dismiss = LocalDismissModal.current
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader("Document")
        Column(Modifier.verticalScroll(rememberScrollState()).padding(20.dp).animateContentSize(Motion.layout()), verticalArrangement = Arrangement.spacedBy(16.dp)) {
            val draft = model.draft
            if (draft == null) {
                Text(model.error ?: "Loading document…", style = Type.body, color = if (model.error == null) Palette.secondary else Palette.danger)
            } else {
                FieldBox(draft.title, { model.edit(title = it) }, "Document title", fill = Palette.surface)
                SegmentedChoice(preview, listOf(false to "Markdown", true to "Preview"), { preview = it })
                if (preview) {
                    if (draft.body.isBlank()) Text("Nothing to preview yet.", style = Type.body, color = Palette.secondary)
                    else MarkdownText(draft.body)
                } else FieldBox(draft.body, { model.edit(body = it) }, "Write Markdown…", 12, 1000, fill = Palette.surface, capitalize = false)
                if (!preview) Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    LinkButton("Heading", enabled = !model.busy) { model.edit(body = draft.body + "\n\n## ") }
                    LinkButton("List", enabled = !model.busy) { model.edit(body = draft.body + "\n\n- ") }
                    LinkButton("Code", enabled = !model.busy) { model.edit(body = draft.body + "\n\n```\n\n```\n") }
                }
                val slash = remember(draft.body) { documentSlash(draft.body) }
                if (!preview) slash?.let { (start, blocks) ->
                    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text("Insert content only. These blocks do not create or run anything.", style = Type.caption, color = Palette.secondary)
                        for (block in blocks) LinkButton("/${block.command} · ${block.label}", enabled = !model.busy) {
                            model.edit(body = draft.body.substring(0, start) + block.markdown)
                        }
                    }
                }
                if (model.links.isNotEmpty() || model.backlinks.isNotEmpty()) {
                    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                        if (model.links.isNotEmpty()) {
                            Text("This page names", style = Type.caption, color = Palette.secondary)
                            for (link in model.links) Text(
                                if (link.documentId == null) "[[${link.target}]] · no page with that title yet" else "[[${link.target}]]",
                                style = Type.caption,
                                color = if (link.documentId == null) Palette.secondary else Palette.text,
                            )
                        }
                        if (model.backlinks.isNotEmpty()) {
                            Text("Linked from", style = Type.caption, color = Palette.secondary)
                            for (page in model.backlinks) Text(page.title, style = Type.caption, color = Palette.text)
                        }
                    }
                }
                if (!preview) Text("Type / on a new final line to insert a block.", style = Type.caption, color = Palette.secondary)
                Text(when {
                    model.saving -> "Saving…"
                    model.conflict != null -> "Another device changed this document. Your draft is kept here."
                    model.error != null -> "Couldn't save. Your draft is kept here."
                    model.dirty -> if (draft.title.isBlank()) "Add a title to save this draft." else "Unsaved changes"
                    draft.id == null -> "New document"
                    else -> "Saved · revision ${draft.ifRevision}"
                }, style = Type.caption, color = Palette.secondary)
                model.error?.let { Text(it, style = Type.body, color = Palette.danger) }
                model.conflict?.let { current ->
                    Text("Current saved revision ${current.revision}: ${current.title}", style = Type.body.semibold, color = Palette.text)
                    SelectionContainer { Text(current.body, style = Type.body, color = Palette.secondary) }
                    SecondaryButton("Reload saved version", Modifier.fillMaxWidth()) { model.reloadConflict() }
                    PrimaryButton("Overwrite with my draft", Modifier.fillMaxWidth(), enabled = !model.saving) { model.overwrite() }
                }
                PrimaryButton("Save", Modifier.fillMaxWidth(), enabled = model.dirty && draft.title.isNotBlank() && !model.saving && model.conflict == null) { model.saveNow() }
                SecondaryButton("Save as a new document", Modifier.fillMaxWidth(), enabled = !model.busy && !model.saving && draft.title.isNotBlank()) { model.saveAsCopy() }
                if (draft.id != null) {
                    val excludedParents = remember(model.items, draft.id) { documentDescendants(model.items, draft.id) }
                    Text("Parent: " + (draft.parentId?.let { id -> model.items.firstOrNull { it.id == id }?.title ?: "Subpage" } ?: "Library root"), style = Type.caption, color = Palette.secondary)
                    DropdownMenu({
                        listOf(MenuItem("Move to library root", enabled = draft.parentId != null) { model.move(null) }) +
                            model.items.filter { it.id !in excludedParents && it.id != draft.parentId }.map { parent ->
                                MenuItem("Move under ${parent.title}") { model.move(parent.id) }
                            }
                    }, enabled = !model.busy && !model.saving && !model.dirty, label = "Move document") {
                        Text("Move document", Modifier.padding(12.dp), style = Type.body, color = Palette.text)
                    }
                    if (model.dirty) Text("Save changes before moving this page.", style = Type.caption, color = Palette.secondary)
                }
                if (model.dirty) SecondaryButton("Discard local draft", Modifier.fillMaxWidth(), enabled = !model.busy && !model.saving) { confirmDiscard = true }
                if (draft.id != null) LinkButton("Delete document and subpages", enabled = !model.busy && !model.saving, color = Palette.danger) { confirmDelete = true }
            }
        }
    }
    CodyncDialog("Delete this document and its subpages?", confirmDelete, { confirmDelete = false }) {
        listOf(DialogAction("Delete", destructive = true) { model.delete { dismiss() } })
    }
    CodyncDialog("Discard unsaved changes?", confirmDiscard, { confirmDiscard = false }) {
        listOf(DialogAction("Discard draft", destructive = true) { model.discardDraft() })
    }
}

@Composable private fun DocumentReviewSheet(model: DocumentsModel) {
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader("Review document")
        Column(Modifier.verticalScroll(rememberScrollState()).padding(20.dp).animateContentSize(Motion.layout()), verticalArrangement = Arrangement.spacedBy(16.dp)) {
            val review = model.review
            val draft = review?.draft
            if (review == null) Text(model.error ?: "Loading exact draft…", style = Type.body, color = Palette.secondary)
            else {
                Text(draft?.title ?: review.title, style = Type.body.semibold, color = Palette.text)
                Text("Status: ${review.status}", style = Type.caption, color = Palette.secondary)
                if (draft != null) {
                    Text("Exact Markdown to be saved · base revision ${draft.ifRevision}", style = Type.caption, color = Palette.secondary)
                    Text(if (draft.id == null) "Create document · parent: ${draft.parentId ?: "Library root"}" else "Update document ${draft.id}", style = Type.caption, color = Palette.secondary)
                    SelectionContainer { Text(draft.body, style = Type.body, color = Palette.text) }
                }
                model.reviewCurrent?.let { current ->
                    Text("Saved version at inspection · revision ${current.revision}", style = Type.body.semibold, color = Palette.text)
                    SelectionContainer { Text(current.body, style = Type.body, color = Palette.secondary) }
                }
                if (review.status == "conflict") Text("The document changed. This proposal was not applied.", style = Type.body, color = Palette.danger)
                review.document?.let { result -> Text("Recorded proposal result · document ${result.id} · revision ${result.revision}", style = Type.caption, color = Palette.secondary) }
                model.error?.let { Text(it, style = Type.body, color = Palette.danger) }
                if (review.status == "pending" && draft != null) {
                    PrimaryButton("Approve and save", Modifier.fillMaxWidth(), enabled = !model.busy) { model.decide(true) }
                    SecondaryButton("Decline", Modifier.fillMaxWidth(), enabled = !model.busy) { model.decide(false) }
                }
            }
        }
    }
}

/** Every page on a circle, with a line for each link that resolves to another page. Tapping one opens it. */
@Composable private fun DocumentGraph(nodes: List<DocumentNode>, edges: List<DocumentEdge>, open: (String) -> Unit) {
    val density = LocalDensity.current
    val line = Palette.border
    val dot = Palette.accent
    BoxWithConstraints(Modifier.fillMaxWidth().semantics { contentDescription = "Links between pages" }) {
        val size = minOf(maxWidth, 420.dp)
        val radius = size.value / 2 - 56
        val center = size.value / 2
        val points = remember(nodes, size) {
            nodes.mapIndexed { index, node ->
                val angle = 2 * PI * index / nodes.size
                node.id to Offset((center + radius * cos(angle)).toFloat(), (center + radius * sin(angle)).toFloat())
            }.toMap()
        }
        Box(Modifier.size(size)) {
            Canvas(Modifier.fillMaxSize()) {
                for (edge in edges) {
                    val from = points[edge.from] ?: continue
                    val to = points[edge.to] ?: continue
                    drawLine(line, from * density.density, to * density.density, 1.dp.toPx())
                }
            }
            for (node in nodes) {
                val point = points[node.id] ?: continue
                Column(
                    Modifier.offset(x = (point.x - 60).dp, y = (point.y - 28).dp)
                        .width(120.dp)
                        .clip(RoundedCornerShape(10.dp))
                        .clickable(role = Role.Button) { open(node.id) }
                        .semantics(mergeDescendants = true) {}
                        .padding(4.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(2.dp),
                ) {
                    Box(Modifier.size(10.dp).clip(CircleShape).background(dot))
                    Text(
                        node.title,
                        style = Type.caption, color = Palette.text, textAlign = TextAlign.Center,
                        maxLines = 2, overflow = TextOverflow.Ellipsis,
                    )
                }
            }
        }
    }
}
