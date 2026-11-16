package com.codync.android.core

import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.jsonObject

/** A durable preference a bot proposes from the person's own messages (`learning`). Nothing is memory until approved. */
@Serializable data class LearningProposal(
    val id: String,
    val botId: String = "",
    val text: String,
    /** `correction`, `instruction` or `repetition`. */
    val kind: String = "instruction",
    /** How many times the person said it. */
    val hits: Int = 1,
    /** The message it came from, cut short. */
    val source: String = "",
    val createdAt: Long = 0,
) {
    /** `Correction`, `Said again · 3 times` or `Preference`. */
    val kindLabel: String get() = when (kind) {
        "correction" -> "Correction"
        "repetition" -> if (hits > 1) "Said again · $hits times" else "Said again"
        else -> "Preference"
    }

    companion object {
        fun decodeAll(response: JsonElement): List<LearningProposal> =
            WireJson.decodeFromJsonElement(kotlinx.serialization.builtins.ListSerializer(serializer()), response.jsonObject.getValue("proposals"))

        fun inboxTitle(count: Int) = if (count > 0) "Learned preferences ($count)" else "Learned preferences"
    }
}

/** The learning screen's state: the setting and the facts waiting for review. */
@Serializable data class LearningInbox(
    val enabled: Boolean = true,
    val autoApproveAfter: Int = 0,
    val maxAutoApproveAfter: Int = 10,
    val note: String = "preferences.md",
    val proposals: List<LearningProposal> = emptyList(),
)
