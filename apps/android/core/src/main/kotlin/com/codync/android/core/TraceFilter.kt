package com.codync.android.core

import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.contentOrNull

/**
 * The typed filters of "Full conversation" (idea 32). Each one is decided by the entry's real event
 * type (`kind`, plus the structured `status` / `style` fields), never by words in its text. A
 * filter only chooses what shows: it never removes entries from the conversation.
 */
enum class TraceFilter(val label: String) {
    All("All"),
    Messages("Messages"),
    Thinking("Thinking"),
    Tools("Tools"),
    Plans("Plans"),
    Approvals("Approvals"),
    Problems("Problems");

    fun matches(entry: Entry): Boolean = when (this) {
        All -> true
        Messages -> entry.kind == "user" || entry.kind == "agent"
        Thinking -> entry.kind == "thought"
        Tools -> entry.kind == "tool"
        Plans -> entry.kind == "plan"
        Approvals -> entry.kind == "permission"
        Problems -> (entry.kind == "tool" && entry.data.field("status") == "failed") ||
            (entry.kind == "notice" && entry.data.field("style") == "error")
    }

    companion object {
        /** How many of [list] each filter shows, for the chips' counts. */
        fun counts(list: List<Entry>): Map<TraceFilter, Int> = TraceFilter.entries.associateWith { filter -> list.count(filter::matches) }

        /**
         * Where to scroll after the turns shown change from [before] to [after] so the reader stays
         * put: the turn they were looking at ([anchor]) if it still shows, else the next shown turn
         * after it, else the last one. Null when nothing shows.
         */
        fun anchorIndex(before: List<Long>, anchor: Long?, after: List<Long>): Int? {
            if (after.isEmpty()) return null
            if (anchor == null || anchor !in before) return after.lastIndex
            val exact = after.indexOf(anchor)
            if (exact >= 0) return exact
            val next = after.indexOfFirst { it > anchor }
            return if (next >= 0) next else after.lastIndex
        }
    }
}

private fun JsonObject.field(key: String): String? = (this[key] as? JsonPrimitive)?.contentOrNull
