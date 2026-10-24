package com.codync.android

import androidx.compose.animation.animateContentSize
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.codync.android.design.*
import com.codync.android.core.Entry

@Composable internal fun VoiceReceipt(entry: Entry, store: AppStore, seconds: Long, transcript: String) {
    var expanded by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    val generation = store.state.collectAsStateWithLifecycle().value.generation
    val model = remember(store, entry.id, generation) { VoiceDecisionsModel(store, entry, scope) }
    LaunchedEffect(model, expanded) { if (expanded) model.load() }
    DisposableEffect(model) { onDispose { model.dispose() } }
    Column(Modifier.fillMaxWidth().animateContentSize(Motion.layoutSize), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("Voice chat · %d:%02d".format(seconds / 60, seconds % 60), style = Type.chatFootnote, color = Palette.secondary)
        SecondaryButton(if (expanded) "Hide call transcript" else "Show call transcript") { expanded = !expanded }
        if (expanded) {
            SelectionContainer { Text(transcript, style = Type.chatFootnote, color = Palette.text) }
            Text("Write only the decisions you personally confirm. Confirming saves an immutable receipt and executes no actions.", style = Type.caption, color = Palette.secondary)
            val confirmed = model.confirmed
            if (confirmed != null) SelectionContainer { Text("Confirmed decisions: $confirmed", style = Type.body, color = Palette.text) }
            else {
                PlainTextField(model.summary, { model.summary = it }, prompt = "Decisions to confirm", enabled = !model.busy, minLines = 2, maxLines = 8)
                SecondaryButton("Confirm these decisions", enabled = !model.loading && !model.busy && model.summary.isNotBlank() && model.summary.length <= 4000) { model.confirm() }
            }
            model.error?.let { Text(it, style = Type.caption, color = Palette.danger) }
        }
    }
}
