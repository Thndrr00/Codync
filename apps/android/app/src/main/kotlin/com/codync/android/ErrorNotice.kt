package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import com.codync.android.core.BotDraft
import com.codync.android.core.Entry
import com.codync.android.design.*
import com.kyant.shapes.RoundedRectangle
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.jsonPrimitive

/** A failed turn can lead straight to the bot's fallback settings, including a group's author. */
@Composable internal fun ErrorNotice(entry: Entry, state: AppState, store: AppStore) {
    var editing by remember(entry.id) { mutableStateOf(false) }
    val bot = state.bots.firstOrNull { it.id == (entry.data.text("author") ?: entry.botId) }
    Column(Modifier.fillMaxWidth().clip(RoundedRectangle(12.dp)).background(Palette.danger.copy(alpha = 0.12f))
        .padding(12.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Symbol("exclamationmark.triangle.fill", size = 16.dp, tint = Palette.danger)
            SelectionContainer { Text(entry.text, style = Type.chatFootnote, color = Palette.text) }
        }
        if (entry.data["addFallback"]?.jsonPrimitive?.booleanOrNull == true && bot != null) {
            SecondaryButton("Add a fallback") { editing = true }
        }
    }
    CodyncSheet(editing, { editing = false }) {
        bot?.let { BotEditor(state, BotDraft.from(it), store) }
    }
}
