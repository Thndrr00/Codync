package com.codync.android.core

/** Keep the default and saved model reachable while searching the host's catalog. */
fun modelSearchChoices(choices: List<ModelChoice>, selection: String, query: String): List<ModelChoice> {
    val search = query.trim()
    val matches = choices.filter { choice ->
        choice.id.isNullOrEmpty() || search.isEmpty() ||
            choice.name.contains(search, ignoreCase = true) ||
            choice.id.contains(search, ignoreCase = true) ||
            choice.description.orEmpty().contains(search, ignoreCase = true)
    }
    if (selection.isEmpty() || matches.any { it.id == selection }) return matches
    val selected = choices.firstOrNull { it.id == selection } ?: ModelChoice(selection, selection)
    return matches + selected
}
