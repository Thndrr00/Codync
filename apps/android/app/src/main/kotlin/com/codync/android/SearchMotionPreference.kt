package com.codync.android

import android.content.Context

/** Device-only search animation preference, independent of the account and host. */
internal object SearchMotionPreference {
    private const val NAME = "codync-search"
    private const val KEY = "animateResults"

    fun read(context: Context): SearchMotionStyle {
        val wire = context.getSharedPreferences(NAME, Context.MODE_PRIVATE).getString(KEY, SearchMotionStyle.Fade.wire)
        return SearchMotionStyle.entries.firstOrNull { it.wire == wire } ?: SearchMotionStyle.Fade
    }

    fun write(context: Context, style: SearchMotionStyle) {
        context.getSharedPreferences(NAME, Context.MODE_PRIVATE).edit().putString(KEY, style.wire).apply()
    }
}
