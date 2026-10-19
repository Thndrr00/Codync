package com.codync.android

import com.codync.android.core.Bot
import com.codync.android.design.MenuItem
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

/** Both roster presentations send the same intents to the host. */
internal fun botRosterActions(bot: Bot, store: AppStore, edit: () -> Unit, delete: () -> Unit): List<MenuItem> =
    listOf(
        MenuItem(if (bot.pinned) "Unpin" else "Pin", icon = if (bot.pinned) "pin.slash" else "pin") {
            store.command("updateBot", buildJsonObject { put("id", bot.id); put("pinned", !bot.pinned) }, "roster:" + bot.id)
        },
        if (bot.kind == "group") MenuItem("Edit group", icon = "person.2", action = edit)
        else MenuItem("Edit profile", icon = "pencil", action = edit),
        MenuItem("Mark as read", icon = "checkmark.message") {
            store.command("markRead", buildJsonObject { put("botId", bot.id); put("all", true) }, "roster:" + bot.id)
        },
        MenuItem("Hide from list", icon = "eye.slash") {
            store.command("updateBot", buildJsonObject { put("id", bot.id); put("hidden", true) }, "roster:" + bot.id)
        },
        MenuItem("Delete", icon = "trash", destructive = true, divider = true, action = delete),
    )
