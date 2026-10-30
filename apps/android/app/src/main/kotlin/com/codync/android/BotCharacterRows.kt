package com.codync.android

import com.codync.android.core.BotDraft
import com.codync.android.core.Personality
import com.codync.android.core.PersonalityLayer
import com.codync.android.design.*

/** One personality field of the editor: its label, help, hint and the layer it edits. */
private data class EditorLayer(val title: String, val detail: String, val hint: String, val layer: PersonalityLayer)

private val LAYERS = listOf(
    EditorLayer("Soul", "Values and boundaries that never change. Written by you; the bot can't edit it.", "e.g. Honest, careful with other people's data.", PersonalityLayer.Soul),
    EditorLayer("Persona", "Its voice and manner.", "e.g. Dry and brief. No emoji.", PersonalityLayer.Persona),
    EditorLayer("Heartbeat", "What it does when it checks in on its own.", "e.g. Look for failing tests and say only if something broke.", PersonalityLayer.Heartbeat),
)

/** What the user writes about how a bot works: its check before done, its soul, persona and heartbeat, its background model. */
internal fun CardRows.characterRows(draft: BotDraft, change: (BotDraft) -> Unit) {
    row {
        Field("Check before done", "A command run in the workspace after each turn. If it fails, the turn doesn't count as done.") {
            FieldBox(draft.verifyCommand.orEmpty(), { change(draft.copy(verifyCommand = it)) }, "e.g. npm test", 1, 3)
        }
    }
    for (layer in LAYERS) row {
        val current = draft.personality ?: Personality()
        Field(layer.title, layer.detail) { FieldBox(layer.layer.of(current), { change(draft.copy(personality = layer.layer.set(current, it))) }, layer.hint, 2, 8) }
    }
    row {
        Field("Background model", "A cheaper model for routines and check-ins, where nobody is waiting. Empty: the bot's own model.") {
            FieldBox(draft.backgroundModel.orEmpty(), { change(draft.copy(backgroundModel = it)) }, "e.g. haiku", 1, 2)
        }
    }
}
