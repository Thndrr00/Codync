package com.codync.android

import com.codync.android.core.BotExchange
import com.codync.android.core.Entry

/** One row of the "Waiting for replies" card above a chat's composer. */
internal data class WaitingReply(val id: String, val name: String, val request: String,
    val stopped: Boolean, val entry: Entry)

/** Derives outstanding "Asked" teammate requests from structured bot messages. A stopped row
 * stays in the list with its new state instead of vanishing. */
internal object WaitingReplies {
    fun rows(entries: List<Entry>, botId: String, threadId: String?, stopped: Set<String>,
             name: (String) -> String): List<WaitingReply> = buildList {
        for (entry in entries) {
            val exchange = BotExchange.of(entry) ?: continue
            if (exchange.chatId != botId || !exchange.isOutgoing || entry.threadId != threadId) continue
            val isStopped = entry.id in stopped
            val asked = entry.data.text("heading")?.startsWith("Asked") == true
            if (!isStopped && (!asked || exchange.outcome != BotExchange.Outcome.Pending)) continue
            add(WaitingReply(entry.id, exchange.peerName ?: name(exchange.peerId), exchange.text, isStopped, entry))
        }
    }.sortedBy { it.id }
}
