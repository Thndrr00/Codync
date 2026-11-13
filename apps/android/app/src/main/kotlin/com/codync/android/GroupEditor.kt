package com.codync.android

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.net.Uri
import android.util.Base64
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateContentSize
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.text.style.TextOverflow
import com.codync.android.core.Bot
import com.codync.android.core.GroupDraft
import com.codync.android.design.*
import java.io.ByteArrayOutputStream

/**
 * Create a group chat, or change its name, what it's for and who's in it (iOS `GroupEditorView`).
 * Bots are picked like recipients: members on top, a search, and the bots not in it yet.
 */
@Composable internal fun GroupEditor(state: AppState, initial: GroupDraft, store: AppStore) {
    val dismiss = LocalDismissModal.current
    var draft by remember(initial.id) { mutableStateOf(initial) }
    var query by remember { mutableStateOf("") }
    // A new group starts in the picker; an existing one shows its members first.
    var adding by remember { mutableStateOf(initial.id == null && initial.members.isEmpty()) }
    var confirmDelete by remember { mutableStateOf(false) }
    val saving = "editor" in state.busy
    val context = LocalContext.current
    var pictureError by remember { mutableStateOf<String?>(null) }
    val pickPicture = rememberLauncherForActivityResult(ActivityResultContracts.PickVisualMedia()) { uri ->
        if (uri != null) {
            val picture = channelPicture(context, uri)
            if (picture != null) {
                pictureError = null
                draft = draft.copy(avatarPhoto = picture)
            } else {
                pictureError = tr("The picture could not be read.")
            }
        }
    }
    val picked = draft.members.mapNotNull { id -> state.bots.firstOrNull { it.id == id } }
    val candidates = roster(state.bots).filter { it.kind != "group" && it.id !in draft.members && it.name.contains(query.trim(), ignoreCase = true) }
    val defaultName = picked.joinToString(", ") { it.name }.ifEmpty { "Name" }
    fun toggle(id: String) {
        draft = draft.copy(members = if (id in draft.members) draft.members - id else draft.members + id)
        query = ""
    }
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader(if (draft.id == null) "New group chat" else "Group chat") {
            if (saving) Box(Modifier.size(36.dp), contentAlignment = Alignment.Center) { Spinner() }
            else IconButton(if (draft.id == null) "Create" else "Save", "checkmark", enabled = draft.members.isNotEmpty()) {
                store.saveGroup(draft) { dismiss() }
            }
        }
        Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(start = 20.dp, end = 20.dp, top = 8.dp, bottom = 24.dp),
            verticalArrangement = Arrangement.spacedBy(28.dp)) {
            Column(Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(10.dp)) {
                GroupAvatar(picked.map { AvatarFace(it.avatarShape, it.avatarColor, it.status) }, 84.dp, photo = draft.avatarPhoto)
                PlainTextField(draft.name, { draft = draft.copy(name = it) }, Modifier.fillMaxWidth().padding(top = 6.dp), defaultName, singleLine = true,
                    style = Type.title2.semibold.copy(textAlign = TextAlign.Center))
                PlainTextField(draft.title, { draft = draft.copy(title = it) }, Modifier.fillMaxWidth(), tr("A short line under the name"), singleLine = true,
                    style = Type.subheadline.copy(textAlign = TextAlign.Center, color = Palette.secondary))
                PlainTextField(draft.description, { draft = draft.copy(description = it) }, Modifier.fillMaxWidth(), "What this group works on",
                    style = Type.subheadline.copy(textAlign = TextAlign.Center, color = Palette.secondary), maxLines = 4)
            }
            Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(14.dp)) {
                ValueRow(tr("Picture"), detail = tr("Shown in the roster instead of the members' faces.")) {
                    Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                        SecondaryButton(if (draft.avatarPhoto == null) tr("Choose Picture") else tr("Change Picture")) {
                            pickPicture.launch(PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly))
                        }
                        if (draft.avatarPhoto != null) SecondaryButton(tr("Remove Picture")) { draft = draft.copy(avatarPhoto = null) }
                    }
                }
                pictureError?.let { Text(it, style = Type.caption, color = Palette.danger) }
                ValueRow(tr("Archived"), detail = tr("Archived groups leave the open roster; their conversation stays.")) {
                    CodyncSwitch(draft.archived, { draft = draft.copy(archived = it) }) {
                        Text(tr("Archived"), style = Type.body)
                    }
                }
            }
            Column(Modifier.fillMaxWidth().animateContentSize(Motion.layoutSize)) {
                Text("Members", Modifier.padding(bottom = 4.dp), style = Type.subheadline.semibold, color = Palette.secondary)
                for (bot in picked) MemberRow(bot, state.bots) { IconButton("Remove ${bot.name}", "xmark") { toggle(bot.id) } }
                if (adding) {
                    Box(Modifier.padding(vertical = 6.dp)) {
                        FieldBox(query, { query = it }, "Search bots", fill = Palette.surface, imeAction = ImeAction.Done,
                            actions = KeyboardActions(onDone = { candidates.firstOrNull()?.let { toggle(it.id) } }))
                    }
                    for (bot in candidates) Box(Modifier.plainButton { toggle(bot.id) }.semantics(mergeDescendants = true) { contentDescription = "Add ${bot.name}" }) {
                        MemberRow(bot, state.bots, dimmed = true) { Symbol("plus.circle", size = 20.dp, tint = Palette.secondary) }
                    }
                } else if (candidates.isNotEmpty()) Row(Modifier.fillMaxWidth().heightIn(min = 48.dp).plainButton { adding = true },
                    verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(14.dp)) {
                    Box(Modifier.width(32.dp), contentAlignment = Alignment.Center) { Symbol("plus", size = 18.dp, tint = Palette.secondary) }
                    Text("Add Member", style = Type.body, color = Palette.secondary)
                }
                Text("Everyone answers in turn unless you @mention someone.", Modifier.padding(top = 10.dp), style = Type.caption, color = Palette.tertiary)
                if (picked.isNotEmpty()) {
                    val chiefOptions = listOf("" to "None (auto-detect Chief)") + picked.map { b -> b.id to (if (b.isChief) "${b.name} ★" else b.name) }
                    Box(Modifier.padding(top = 14.dp)) {
                        ValueRow("Chief of Staff", detail = "Coordinates delegated assignments and automatically intervenes if a member stalls or fails.") {
                            ChoicePicker(draft.chiefBotId.orEmpty(), chiefOptions, { draft = draft.copy(chiefBotId = it.ifEmpty { null }) }, label = "Chief of Staff")
                        }
                    }
                }
            }
            AnimatedVisibility(state.error != null, enter = fadeIn(Motion.fade()), exit = fadeOut(Motion.fade())) {
                Text(state.error.orEmpty(), style = Type.footnote, color = Palette.danger)
            }
            draft.id?.let { DocumentsCard(it, state, store) }
            draft.id?.let { id -> state.bots.firstOrNull { it.id == id }?.let { HistoryActions(it, store) } }
            if (draft.id != null && draft.archived) SecondaryButton(tr("Restore group chat")) {
                draft = draft.copy(archived = false)
                store.saveGroup(draft) { dismiss() }
            }
            if (draft.id != null) LinkButton("Delete group chat", color = Palette.danger) { confirmDelete = true }
        }
    }
    val group = state.bots.firstOrNull { it.id == draft.id }
    CodyncDialog("Delete ${group?.name ?: "group chat"}?", confirmDelete, { confirmDelete = false }, "Its bots and their own chats stay.") {
        listOf(DialogAction("Delete group chat", destructive = true) { group?.let { store.deleteBot(it) { dismiss() } } })
    }
}

/** A member or candidate: avatar, name, and a trailing control. */
@Composable private fun MemberRow(bot: Bot, bots: List<Bot>, dimmed: Boolean = false, trailing: @Composable () -> Unit) {
    Row(Modifier.fillMaxWidth().heightIn(min = 48.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(14.dp)) {
        BotAvatar(bot, bots, 32.dp)
        Text(bot.name, Modifier.weight(1f), style = Type.body, color = if (dimmed) Palette.secondary else Palette.text, maxLines = 1, overflow = TextOverflow.Ellipsis)
        trailing()
    }
}

/** The picked photo as a channel picture: 512 pixels on the long side, JPEG, a data URL (the host keeps 300 KB). */
private fun channelPicture(context: Context, uri: Uri): String? {
    val bytes = runCatching { context.contentResolver.openInputStream(uri)?.use { it.readBytes() } }.getOrNull() ?: return null
    val bitmap = BitmapFactory.decodeByteArray(bytes, 0, bytes.size) ?: return null
    val scale = minOf(512f / bitmap.width, 512f / bitmap.height, 1f)
    val picture = if (scale < 1f)
        Bitmap.createScaledBitmap(bitmap, (bitmap.width * scale).toInt(), (bitmap.height * scale).toInt(), true)
    else bitmap
    val out = ByteArrayOutputStream()
    picture.compress(Bitmap.CompressFormat.JPEG, 80, out)
    return "data:image/jpeg;base64," + Base64.encodeToString(out.toByteArray(), Base64.NO_WRAP)
}
