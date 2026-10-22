package com.codync.android.core

/**
 * The "N new" on Jump to latest (idea 31) for one conversation (one bot's chat or one thread; make one
 * counter per scope). It counts distinct entry ids that appear after the newest message the reader
 * had when they left the bottom, so a streaming edit (same id), an older-history load (ids before
 * that message) and the reader's own messages ([own]) never add to it. Back at the bottom it resets.
 */
class NewMessageCounter {
    private var anchor: String? = null
    private val known = HashSet<String>()
    private val fresh = LinkedHashSet<String>()

    /** How many new messages arrived since the reader left the bottom. */
    val count: Int get() = fresh.size

    /**
     * Feeds the conversation's message ids, oldest first, and whether the reader is on the newest
     * message. Returns the new count.
     */
    fun update(ids: List<String>, atBottom: Boolean, own: Set<String> = emptySet()): Int {
        if (atBottom || ids.isEmpty()) { reset(); return 0 }
        if (anchor == null) {
            anchor = ids.last()
            known += ids
            return 0
        }
        val anchorIndex = ids.lastIndexOf(anchor)
        // Entries after the anchor are new; if the anchor itself is gone, only ids never seen count.
        val candidates = if (anchorIndex >= 0) ids.subList(anchorIndex + 1, ids.size) else ids
        for (id in candidates) if (id !in known && id !in own) fresh += id
        known += ids
        return fresh.size
    }

    fun reset() {
        anchor = null
        known.clear()
        fresh.clear()
    }
}
