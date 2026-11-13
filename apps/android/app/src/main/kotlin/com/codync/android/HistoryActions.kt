package com.codync.android

import androidx.compose.runtime.*
import com.codync.android.core.Bot
import com.codync.android.design.*

/**
 * A chat's or group's history actions, in its settings: Delete history removes its messages and turns
 * only; New conversation keeps them and starts the agents fresh; Compact context asks the agent (or
 * every member of a group) to summarize what it keeps. Ported from OpenBot.
 */
@Composable
internal fun HistoryActions(bot: Bot, store: AppStore) {
    var confirming by remember { mutableStateOf(false) }
    var typed by remember { mutableStateOf("") }
    CardSection("History") {
        LinkButton(tr("Delete history"), color = Palette.danger) {
            typed = ""
            confirming = true
        }
        LinkButton(tr("New conversation")) { store.command("newSession", body("botId" to bot.id)) }
        LinkButton(tr("Compact context")) { store.compactContext(bot) }
    }
    CodyncDialog(
        "Delete ${bot.name}'s history?",
        confirming,
        { confirming = false },
        message = "Its messages and turns go. Its setup, memories, routines, artifacts and files stay, and no other chat changes.",
        field = { PlainInput(typed, { typed = it }, "Type ${bot.name} to confirm") },
    ) {
        listOf(
            DialogAction("Delete history", destructive = true, enabled = typed.trim() == bot.name) {
                store.deleteHistory(bot)
            },
        )
    }
}
