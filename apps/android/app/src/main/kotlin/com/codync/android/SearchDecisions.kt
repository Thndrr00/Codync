package com.codync.android

/** The device's "Animate search results" choice. Labels are the settings row's options. */
internal enum class SearchMotionStyle(val wire: String, val label: String) {
    Blur("blur", "Blur"), Fade("fade", "Fade"), None("none", "None");

    companion object {
        const val TITLE = "Animate search results"
    }
}

/** The three rows Settings shows for the device's search animation, in a stable order. */
internal fun searchMotionOptions(): List<Pair<SearchMotionStyle, String>> =
    SearchMotionStyle.entries.map { it to it.label }

/** What a search row actually animates with: blur is an iOS veil, so Android only fades. */
internal enum class SearchMotion { Fade, None }

/** Pure: Reduce Motion and "None" switch animation off; Android has no blur veil (iOS does). */
internal fun searchMotion(style: SearchMotionStyle, reduce: Boolean): SearchMotion =
    if (reduce || style == SearchMotionStyle.None) SearchMotion.None else SearchMotion.Fade

/** Pure: the Searching spinner appears only after the host has had [delayMs] to answer. */
internal fun showSpinner(startedAt: Long?, now: Long, delayMs: Long = 200L): Boolean =
    startedAt != null && now - startedAt >= delayMs

/** Pure: keep the last page while an edited query loads; a cleared field's first word starts empty. */
internal fun hitsAfter(queryChanged: Boolean, cleared: Boolean): Boolean = !(queryChanged && cleared)
