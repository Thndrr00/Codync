package com.codync.android

import com.codync.android.core.BotDraft

/** Hello updates may pick a default until the person chooses their agent or model. */
internal class NewBotAgentDefaults {
    private var selected = false

    fun edited(before: BotDraft, after: BotDraft): BotDraft {
        selected = selected || before.backend != after.backend || before.model != after.model ||
            before.account != after.account || before.modelOptions != after.modelOptions
        return after
    }

    fun refreshed(draft: BotDraft, state: AppState): BotDraft =
        if (selected) draft else withKnownAgent(draft, state)
}
