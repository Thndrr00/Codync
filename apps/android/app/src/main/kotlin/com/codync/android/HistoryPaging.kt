package com.codync.android

/**
 * Whether the chat should ask for an earlier page now: the list has settled with the "earlier" row on screen,
 * nothing is loading, and the last page asked for moved the chat at all (`shown` rows, the oldest `floor`:
 * more rows or an older floor). A page of only tool or thought rows moves neither, so it isn't asked twice.
 */
internal fun pageEarlier(settled: Boolean, visible: Boolean, loading: Boolean, shown: Int, floor: Long,
    askedShown: Int, askedFloor: Long): Boolean =
    settled && visible && !loading && (shown != askedShown || floor < askedFloor)

/**
 * The chat's earlier pages, one request at a time (iOS `canLoadOlder` with the row's `task`): a request only while
 * the row is on screen and the list settled, and after a failed page only once the row left and came back, so a
 * computer that keeps failing isn't asked in a loop.
 */
internal class EarlierPaging {
    private var askedShown = -1
    private var askedFloor = Long.MAX_VALUE
    /** The row left the screen since the last request. */
    private var left = false
    /** The last request failed; a new one waits for the row to come back first. */
    private var failed = false

    /** The row left the list: coming back, it may ask again for a page that failed. */
    fun left() { left = true }

    /** The last request failed. */
    fun failed() { failed = true }

    /** Records and answers whether the chat should ask for a page now; see [pageEarlier]. */
    fun ask(settled: Boolean, visible: Boolean, loading: Boolean, shown: Int, floor: Long): Boolean {
        if (!visible) { left = true; return false }
        if (!settled || loading) return false
        if (!(failed && left) && !pageEarlier(settled, visible, loading, shown, floor, askedShown, askedFloor)) return false
        askedShown = shown
        askedFloor = floor
        failed = false
        left = false
        return true
    }
}
