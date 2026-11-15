package com.codync.android.core

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/** Where a mission is: proposed until somebody takes it on, then running, waiting, finished, or dropped. */
@Serializable
enum class MissionStatus {
    @SerialName("proposed") PROPOSED,
    @SerialName("active") ACTIVE,
    @SerialName("waiting") WAITING,
    @SerialName("done") DONE,
    @SerialName("abandoned") ABANDONED;

    /** A finished or dropped mission: it stays in the list only when asked for. */
    val returned: Boolean get() = this == DONE || this == ABANDONED

    /** The status somebody pushing this mission on moves it to, if there is such a move. */
    val next: MissionStatus? get() = if (this == ACTIVE) WAITING else ACTIVE

    /** The verb of that move. */
    val nextTitle: String get() = when (this) {
        PROPOSED -> "Activate"
        ACTIVE -> "Pause"
        WAITING -> "Resume"
        DONE, ABANDONED -> "Reopen"
    }

    val label: String get() = when (this) {
        PROPOSED -> "Proposed"
        ACTIVE -> "Running"
        WAITING -> "Waiting"
        DONE -> "Done"
        ABANDONED -> "Abandoned"
    }
}

@Serializable
enum class MissionStepStatus {
    @SerialName("pending") PENDING,
    @SerialName("active") ACTIVE,
    @SerialName("done") DONE,
    @SerialName("skipped") SKIPPED;

    /** The glyph a step shows as. */
    val mark: String get() = when (this) {
        PENDING -> "·"
        ACTIVE -> "▸"
        DONE -> "✓"
        SKIPPED -> "–"
    }
}

@Serializable
data class MissionStep(
    val id: String,
    val missionId: String = "",
    val position: Int = 0,
    val text: String = "",
    val status: MissionStepStatus = MissionStepStatus.PENDING,
    val at: Long = 0L,
)

/** An objective that outlives a turn: what is done, what is left, and why it stopped. */
@Serializable
data class Mission(
    val id: String,
    val botId: String = "",
    val title: String = "",
    val goal: String = "",
    val status: MissionStatus = MissionStatus.PROPOSED,
    val stopReason: String? = null,
    val checkpoint: String = "",
    val runId: String? = null,
    val approvalId: String? = null,
    val createdAt: Long = 0L,
    val updatedAt: Long = 0L,
    val steps: List<MissionStep> = emptyList(),
)
