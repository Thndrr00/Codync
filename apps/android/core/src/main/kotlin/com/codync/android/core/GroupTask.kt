package com.codync.android.core

import kotlinx.serialization.Serializable

/**
 * One row of a group's request tree (`chat/group/tasks.rs`): the request, or a member's assignment under it. Names
 * match the host's camelCase JSON.
 */
@Serializable
data class GroupTask(
    val id: String,
    val groupId: String = "",
    /** The lane's key: the chat, or its thread. */
    val lane: String = "",
    /** The user's message the tree answers. */
    val requestId: String = "",
    val rootId: String = "",
    val parentId: String? = null,
    /** The member the row belongs to; null for the request of a channel with no lead. */
    val ownerId: String? = null,
    val ownerName: String? = null,
    /** `request` or `assignment`. */
    val kind: String = "assignment",
    /** `queued`, `running`, `completed`, `failed`, `paused` or `cancelled`. */
    val state: String = "queued",
    val instruction: String = "",
    /** Why it stopped or failed. */
    val note: String? = null,
    val createdAt: Long = 0,
    val updatedAt: Long = 0,
) {
    /** Still part of the request's work: it may take a turn, or wait for one. */
    val isOpen: Boolean get() = state in listOf("queued", "running", "paused", "failed")

    /** Stopped or failed: it offers Continue, Reassign and Cancel. */
    val isStopped: Boolean get() = state == "paused" || state == "failed"
}
