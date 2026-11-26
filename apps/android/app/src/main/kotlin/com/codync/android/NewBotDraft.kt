package com.codync.android

import android.content.Context
import com.codync.android.core.BotDraft
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

/** Keep the chosen model and reasoning option when the New bot screen is left and reopened. */
internal object NewBotDraftKeep {
    fun keep(saved: BotDraft?, fresh: BotDraft): BotDraft {
        if (saved == null) return fresh
        val model = fresh.model?.takeIf { it.isNotBlank() } ?: saved.model
        val options = fresh.modelOptions.toMutableMap()
        for ((id, value) in saved.modelOptions) if (options[id].isNullOrBlank()) options[id] = value
        return fresh.copy(model = model, modelOptions = options.toMap())
    }
}

/** The phone's copy of the desktop's per-device new-bot draft. */
internal class NewBotDraftStore(context: Context) {
    private val prefs = context.applicationContext.getSharedPreferences("codync-new-bot", Context.MODE_PRIVATE)

    var saved: BotDraft?
        get() = prefs.getString("draft", null)?.let { runCatching { Json.decodeFromString<BotDraft>(it) }.getOrNull() }
        set(value) = prefs.edit().run {
            if (value == null) remove("draft") else putString("draft", Json.encodeToString(value))
            apply()
        }
}
