package com.codync.android.core

import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.jsonObject

/** The GitHub account of the computer's host, as `githubStatus` reports it. Never carries the token. */
@Serializable data class GitHubStatus(
    val configured: Boolean = false,
    val login: String? = null,
    val scopes: List<String> = emptyList(),
)

/** One installation of the host's GitHub App and the repositories it may reach. */
@Serializable data class GitHubInstallation(
    val id: String = "",
    val account: String = "",
    val repos: List<String> = emptyList(),
)

/** A repository that reaches bots, on the branches listed (none means all of them). */
@Serializable data class GitHubRepoMap(
    val repo: String = "",
    val bots: List<String> = emptyList(),
    val branches: List<String> = emptyList(),
)

/** The GitHub App of the computer's host, as `githubAppStatus` reports it. Never carries the key or secret. */
@Serializable data class GitHubAppStatus(
    val configured: Boolean = false,
    val appId: String? = null,
    val slug: String? = null,
    val installations: List<GitHubInstallation> = emptyList(),
    val repos: List<GitHubRepoMap> = emptyList(),
)

/** What the GitHub settings card shows. */
object GitHub {
    fun decode(response: JsonElement): GitHubStatus =
        WireJson.decodeFromJsonElement(GitHubStatus.serializer(), response.jsonObject)

    fun decodeApp(response: JsonElement): GitHubAppStatus =
        WireJson.decodeFromJsonElement(GitHubAppStatus.serializer(), response.jsonObject)
}
