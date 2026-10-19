package com.codync.android

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

internal enum class RosterPresentation { List, Constellation }

/** Device-only presentation preference; no bot state or host commands belong here. */
internal class RosterPreferences(context: Context) {
    private val preferences = context.getSharedPreferences("roster-presentation", Context.MODE_PRIVATE)
    var presentation by mutableStateOf(
        if (preferences.getBoolean("constellation", false)) RosterPresentation.Constellation else RosterPresentation.List)
        private set

    fun select(value: RosterPresentation) {
        presentation = value
        preferences.edit().putBoolean("constellation", value == RosterPresentation.Constellation).apply()
    }
}
