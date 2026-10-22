package com.codync.android.core

/** One launcher shortcut to a bot: a stable ID, so a rename updates it instead of adding another. */
data class BotShortcut(val id: String, val botId: String, val label: String)

/**
 * Which bots the launcher offers. Only a few pinned bots, in roster order: the launcher decides how many fit.
 * An ID names account, computer and bot, so a shortcut that outlives them can be told apart when it is opened.
 */
object BotShortcutPlan {
    const val PREFIX = "launch:"

    fun id(scope: String, computerId: String, botId: String): String = "$PREFIX$scope/$computerId/$botId"

    fun plan(scope: String, computerId: String, bots: List<Bot>, limit: Int): List<BotShortcut> =
        bots.filter { it.pinned && !it.hidden && !it.deleted && it.name.isNotBlank() }
            .take(limit.coerceAtLeast(0))
            .map { BotShortcut(id(scope, computerId, it.id), it.id, it.name.take(25)) }
}
