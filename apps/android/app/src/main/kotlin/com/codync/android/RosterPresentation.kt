package com.codync.android

import android.content.Context
import android.content.SharedPreferences
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

internal enum class RosterPresentation { List, Constellation }

/** Device-only presentation and grouping preferences; no bot state or host commands belong here. */
internal class RosterPreferences internal constructor(private val preferences: SharedPreferences) {
    constructor(context: Context) : this(
        context.getSharedPreferences("roster-presentation", Context.MODE_PRIVATE),
    )

    var presentation by mutableStateOf(
        if (preferences.getBoolean("constellation", false)) RosterPresentation.Constellation else RosterPresentation.List,
    )
        private set

    var groupByProject by mutableStateOf(preferences.getBoolean("groupByProject", false))
        private set

    private var collapsedKeys by mutableStateOf(
        preferences.getStringSet("collapsedSections", emptySet()).orEmpty().toSet(),
    )

    fun select(value: RosterPresentation) {
        presentation = value
        preferences.edit().putBoolean("constellation", value == RosterPresentation.Constellation).apply()
    }

    fun updateGroupByProject(enabled: Boolean) {
        groupByProject = enabled
        preferences.edit().putBoolean("groupByProject", enabled).apply()
    }

    fun isSectionCollapsed(contextId: String, key: WorkspaceKey): Boolean =
        "$contextId|${key.storageKey}" in collapsedKeys

    fun toggleSectionCollapsed(contextId: String, key: WorkspaceKey) {
        val token = "$contextId|${key.storageKey}"
        val updated = if (token in collapsedKeys) collapsedKeys - token else collapsedKeys + token
        collapsedKeys = updated
        preferences.edit().putStringSet("collapsedSections", updated).apply()
    }
}
