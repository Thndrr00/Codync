package com.codync.android.core

import kotlinx.serialization.Serializable

/** Settings that belong to a computer rather than to one bot (`serverSettings`). */
@Serializable
data class ServerSettings(
    /** `natural`: a short note only when a finding changes the plan or before a long step. `classic`: the agent's own text. */
    val responseStyle: String = "natural",
    /** Claude agents load the person's own `~/.claude` (CLAUDE.md, skills, agents, hooks, plugins). Off by default. */
    val claudeUserSetup: Boolean = false,
    val remoteControl: RemoteControl = RemoteControl(),
)

@Serializable
data class RemoteControl(
    /** `everyone`, `admins` (reads as everyone until member roles exist) or `owner` (this computer only). */
    val access: String = "everyone",
    /** The person at the computer allows each session; one nobody allows in 30 seconds does not start. */
    val askOnHost: Boolean = false,
)
