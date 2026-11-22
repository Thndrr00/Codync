package com.codync.android

import com.codync.android.core.ThreadInfo

/** Paging for the archived-thread drawer: a full page means another one may exist. */
internal object ThreadPaging {
    const val pageSize = 50

    /** Merges a host page into the rows already shown. An older host that ignores the cursor
     * keeps returning the same rows, so paging stops when nothing new arrives. */
    fun merge(existing: List<ThreadInfo>, incoming: List<ThreadInfo>): Pair<List<ThreadInfo>, Boolean> {
        val known = existing.map { it.id }.toMutableSet()
        val added = incoming.filter { known.add(it.id) }
        return (existing + added).sortedByDescending { it.updatedAt } to (incoming.size >= pageSize && added.isNotEmpty())
    }

    /** The cursor for the next page: the oldest row already shown. */
    fun cursor(threads: List<ThreadInfo>): String? = threads.lastOrNull()?.id
}
