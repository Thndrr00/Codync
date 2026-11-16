package com.codync.android.core

import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.jsonObject

/** A repository `githubCreateRepo` just made. Only a create answer carries it. */
@Serializable data class CreatedRepo(
    val fullName: String = "",
    val htmlUrl: String = "",
    val cloneUrl: String = "",
    /** The host pointed the workspace at it, because the workspace had no origin. */
    val originSet: Boolean = false,
)

/** The GitHub account of the computer's host, as `githubStatus` reports it. Never carries the token. */
@Serializable data class GitHubStatus(
    val configured: Boolean = false,
    val login: String? = null,
    val scopes: List<String> = emptyList(),
    /** The token may create repositories (Administration, or a classic token's `repo` scope). */
    val canCreateRepos: Boolean = false,
    val created: CreatedRepo? = null,
)

/** What the GitHub settings card shows. */
object GitHub {
    /** `github_pat_…` or a classic token: 20–255 letters, digits, `-` or `_`. */
    fun tokenOk(token: String): Boolean {
        val value = token.trim()
        return value.length in 20..255 && value.all { it in 'a'..'z' || it in 'A'..'Z' || it in '0'..'9' || it == '_' || it == '-' }
    }

    /** GitHub's own repository-name rule, checked before the host is asked. */
    fun repoNameOk(name: String): Boolean {
        val value = name.trim()
        return value.isNotEmpty() && value.length <= 100 && value.first().let {
            it in 'a'..'z' || it in 'A'..'Z' || it in '0'..'9' || it == '_' || it == '-'
        } && value.all { it in 'a'..'z' || it in 'A'..'Z' || it in '0'..'9' || it == '_' || it == '-' || it == '.' }
    }

    fun decode(response: JsonElement): GitHubStatus =
        WireJson.decodeFromJsonElement(GitHubStatus.serializer(), response.jsonObject)
}
