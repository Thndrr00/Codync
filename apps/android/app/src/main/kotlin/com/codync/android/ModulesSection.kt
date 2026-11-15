package com.codync.android

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import com.codync.android.core.HostModule
import com.codync.android.core.ModulePermissions
import com.codync.android.core.ModuleScreen
import com.codync.android.core.WireJson
import com.codync.android.design.CardSection
import com.codync.android.design.CodyncDialog
import com.codync.android.design.CodyncSheet
import com.codync.android.design.DialogAction
import com.codync.android.design.IconButton
import com.codync.android.design.ModalHeader
import com.codync.android.design.Palette
import com.codync.android.design.SecondaryButton
import com.codync.android.design.Type
import com.codync.android.design.row
import java.io.File
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.JsonElement

/**
 * Extensions › Modules: what a person added to Codync beyond what it ships with. Each one is a signed folder the
 * host runs in its own process; nothing it offers a bot runs until the owner approves it here.
 */
@Composable
internal fun ModulesSection(query: String, store: AppStore) {
    val scope = rememberCoroutineScope()
    val context = LocalContext.current
    var modules by remember { mutableStateOf<List<HostModule>?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var notice by remember { mutableStateOf<String?>(null) }
    var open by remember { mutableStateOf<HostModule?>(null) }
    var importing by remember { mutableStateOf(false) }

    LaunchedEffect(store) {
        runCatching { store.modules() }
            .onSuccess { modules = it }
            .onFailure { error = it.message ?: "Couldn't load modules." }
    }

    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        if (uri == null) return@rememberLauncherForActivityResult
        scope.launch {
            try {
                val text = withContext(Dispatchers.IO) {
                    context.contentResolver.openInputStream(uri)?.use { it.readBytes() }?.decodeToString()
                } ?: return@launch
                val bundle = WireJson.parseToJsonElement(text)
                modules = store.installModule(bundle)
                notice = "Installed. Approve what it asks for to use it."
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { notice = failure.message ?: "That file is not a module bundle." }
        }
    }

    val shown = (modules ?: emptyList()).filter {
        query.isEmpty() || it.name.contains(query, ignoreCase = true) || it.description.contains(query, ignoreCase = true)
    }
    error?.let { Text(it, style = Type.footnote, color = Palette.danger) }
    CardSection("On this computer") {
        if (modules != null && shown.isEmpty()) row {
            Text("No modules yet. A module adds tools and a screen to Codync, signed by whoever made it.",
                style = Type.footnote, color = Palette.secondary)
        }
        for (module in shown) row {
            MarketRow(module.name, subtitle(module), added = module.enabled, open = { open = module },
                icon = { TileIcon(module.symbol ?: "puzzlepiece.extension") }) { open = module }
        }
        row {
            SecondaryButton("Install from a file", enabled = !importing) {
                importing = true
                picker.launch(arrayOf("*/*"))
            }
            notice?.let { Text(it, style = Type.caption, color = Palette.secondary) }
        }
    }

    open?.let { module ->
        ModuleSheet(
            module = module,
            store = store,
            apply = { list ->
                modules = list
                open = list.firstOrNull { it.id == module.id }
            },
            onClose = { open = null },
        )
    }
}

/** The line under a module's name: its version, its state and what was approved. */
private fun subtitle(module: HostModule): String = buildList {
    module.version?.let { add("v$it") }
    add(module.stateText)
    add(module.approvedText)
}.joinToString(" · ")

/** One module: what it asks for, what was approved, its switch, its screens, and taking it off. */
@Composable
private fun ModuleSheet(module: HostModule, store: AppStore, apply: (List<HostModule>) -> Unit, onClose: () -> Unit) {
    val scope = rememberCoroutineScope()
    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var screen by remember { mutableStateOf<ModuleScreen?>(null) }
    var removing by remember { mutableStateOf(false) }
    val nothingApproved = module.approved.tools.isEmpty() && module.approved.events.isEmpty() && module.approved.files == "none"

    fun change(work: suspend () -> List<HostModule>) {
        if (busy) return
        busy = true; error = null
        scope.launch {
            try { apply(work()) }
            catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { error = failure.message ?: "Couldn't change the module." }
            finally { busy = false }
        }
    }

    CodyncSheet(isPresented = true, onDismiss = onClose) {
        Column(Modifier.fillMaxSize()) {
            ModalHeader(module.name) { IconButton("Close", "xmark") { onClose() } }
            Column(Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = 16.dp, vertical = 12.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp)) {
                module.broken?.let { broken ->
                    CardSection("It can't run") { row { Text(broken, style = Type.footnote, color = Palette.danger) } }
                }
                CardSection(module.description.takeIf { it.isNotEmpty() }) {
                    row {
                        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                            Text(module.version?.let { "v$it" } ?: module.id, style = Type.body, color = Palette.text)
                            Text("${module.stateText} · ${module.approvedText}", style = Type.caption, color = Palette.secondary)
                        }
                        SecondaryButton(if (module.enabled) "Turn off" else "Turn on", enabled = !busy && module.broken == null) {
                            change { store.setModule(module.id, !module.enabled) }
                        }
                    }
                    module.stopped?.let { Text("Last stopped: $it", style = Type.caption, color = Palette.secondary) }
                }
                module.permissions?.let { permissions ->
                    CardSection(if (nothingApproved) "Approve what this module may do. A bot can only call what you approve."
                        else "Approved: ${module.approvedText}.") {
                        row { PermissionLine("Tools", permissions.tools, module.approved.tools) }
                        row { PermissionLine("Events", permissions.events, module.approved.events) }
                        row { PermissionLine("Hosts", permissions.network, module.approved.network) }
                        row {
                            Text("Files", style = Type.footnote, color = Palette.text)
                            Text(if (permissions.files == "none") "None"
                                else "${if (module.approved.files == "none") "·" else "✓"} ${permissions.files}",
                                style = Type.caption, color = Palette.secondary)
                        }
                        row {
                            SecondaryButton(if (nothingApproved) "Approve everything it asks for" else "Approve all of it",
                                enabled = !busy) { change { store.approveModule(module.id) } }
                            if (!nothingApproved) SecondaryButton("Approve nothing", enabled = !busy) {
                                change { store.approveModule(module.id, ModulePermissions()) }
                            }
                        }
                    }
                }
                if (module.views.isNotEmpty()) {
                    CardSection("Screens") {
                        row {
                            for (view in module.views) SecondaryButton(view.title, enabled = module.enabled) {
                                scope.launch {
                                    try { screen = store.moduleView(module.id, view.id) }
                                    catch (cancelled: CancellationException) { throw cancelled }
                                    catch (failure: Exception) { error = failure.message }
                                }
                            }
                        }
                        screen?.let { shown ->
                            for (section in shown.sections) {
                                row {
                                    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                                        Text(section.title, style = Type.caption, color = Palette.secondary)
                                        for (line in section.rows) {
                                            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                                                Text(line.label, style = Type.caption, color = Palette.secondary)
                                                Text(line.value, style = Type.caption, color = Palette.text)
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        error?.let { Text(it, style = Type.caption, color = Palette.danger) }
                    }
                }
                CardSection(module.folder) {
                    row {
                        SecondaryButton("Save to a file", enabled = !busy) {
                            scope.launch {
                                try {
                                    val saved = store.exportModule(module.id)
                                    val folder = withContext(Dispatchers.IO) { store.attachments.shareFolder().apply { mkdirs() } }
                                    val file = File(folder, saved.name)
                                    withContext(Dispatchers.IO) {
                                        file.writeText(WireJson.encodeToString(JsonElement.serializer(), saved.bundle))
                                    }
                                    store.attachments.open(file, file.name)
                                } catch (cancelled: CancellationException) { throw cancelled }
                                catch (failure: Exception) { error = failure.message ?: "Couldn't save it." }
                            }
                        }
                        SecondaryButton("Remove", enabled = !busy) { removing = true }
                    }
                    Text("Removing takes its files off this computer; the bots keep working without its tools.",
                        style = Type.caption, color = Palette.tertiary)
                }
            }
        }
        CodyncDialog("Remove ${module.name}?", isPresented = removing, onDismiss = { removing = false },
            message = "Its files leave this computer and the bots lose its tools. What the bots already did stays.") {
            listOf(
                DialogAction("Remove", destructive = true) {
                    removing = false
                    change {
                        val list = store.uninstallModule(module.id)
                        onClose()
                        list
                    }
                },
            )
        }
    }
}

/** One permission, with a check on each item the owner approved. */
@Composable
private fun PermissionLine(label: String, asked: List<String>, allowed: List<String>) {
    Text(label, style = Type.footnote, color = Palette.text)
    Text(if (asked.isEmpty()) "Nothing" else asked.joinToString("  ") { "${if (allowed.contains(it)) "✓" else "·"} $it" },
        style = Type.caption, color = Palette.secondary)
}
