package com.codync.android

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.unit.dp
import com.codync.android.core.LearningInbox
import com.codync.android.core.LearningProposal
import com.codync.android.design.*
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.put
import kotlinx.serialization.json.JsonPrimitive

/** The learning inbox; null from an older host. */
suspend fun AppStore.learning(method: String, botId: String, id: String? = null, enabled: Boolean? = null, auto: Int? = null): LearningInbox? =
    runCatching {
        val response: JsonElement = hostQuery(method, buildJsonObject {
            put("botId", botId)
            id?.let { put("id", it) }
            enabled?.let { put("enabled", it) }
            auto?.let { put("autoApproveAfter", it) }
        })
        LearningInbox(
            enabled = response.jsonObject["enabled"]?.let { it as? JsonPrimitive }?.content?.toBooleanStrictOrNull() ?: true,
            autoApproveAfter = response.jsonObject["autoApproveAfter"]?.let { (it as? JsonPrimitive)?.content?.toIntOrNull() } ?: 0,
            maxAutoApproveAfter = response.jsonObject["maxAutoApproveAfter"]?.let { (it as? JsonPrimitive)?.content?.toIntOrNull() } ?: 10,
            note = response.jsonObject["note"]?.let { (it as? JsonPrimitive)?.content } ?: "preferences.md",
            proposals = LearningProposal.decodeAll(response),
        )
    }.getOrNull()

/** What a bot learned from the person's corrections and repeated instructions: one card each, saved only on approval. */
@Composable internal fun LearningCard(botId: String, store: AppStore) {
    var inbox by remember(botId) { mutableStateOf<LearningInbox?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    LaunchedEffect(botId) { inbox = store.learning("learning", botId) }
    fun act(method: String, id: String? = null, enabled: Boolean? = null, auto: Int? = null) {
        scope.launch {
            val next = store.learning(method, botId, id, enabled, auto)
            if (next == null) error = "That did not work." else { inbox = next; error = null }
        }
    }
    val current = inbox ?: return
    CardSection(LearningProposal.inboxTitle(current.proposals.size),
        footer = "Corrections and instructions you give this bot can become one line of its ${current.note} note. Nothing is saved until you approve it, unless you let it save after repeats. The bot's replies, routines and group chats are never read for this.") {
        error?.let { row { Text(it, style = Type.caption, color = Palette.danger) } }
        row { SwitchRow("Learn from my corrections and instructions", "Proposes durable preferences, never one-off tasks or secrets.", current.enabled, { act("saveLearningSettings", enabled = it) }, enabled = true) }
        row {
            SegmentedChoice(current.autoApproveAfter, listOf(0 to "Off", 2 to "2×", 3 to "3×", 5 to "5×"), { act("saveLearningSettings", auto = it) })
        }
        row { Text("Save without review after", style = Type.caption, color = Palette.secondary) }
        if (current.proposals.isEmpty()) row { Text(tr("Nothing to review."), style = Type.body, color = Palette.secondary) }
        for (p in current.proposals) row {
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text(p.text, style = Type.body, color = Palette.text)
                Text(p.kindLabel, style = Type.caption, color = Palette.tertiary)
                if (p.source.isNotBlank()) Text("${tr("From")}: “${p.source}”", style = Type.footnote, color = Palette.secondary, maxLines = 2)
                SecondaryButton(tr("Remember this")) { act("approveLearning", p.id) }
                SecondaryButton(tr("Dismiss")) { act("dismissLearning", p.id) }
            }
        }
    }
}
