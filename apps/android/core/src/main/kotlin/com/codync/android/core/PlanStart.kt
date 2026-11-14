package com.codync.android.core

import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement

/**
 * When a plan card of a terminal agent starts the plan by itself (`planStart`), and what it may be.
 * `afterMs` null waits until the person presses Start.
 */
@Serializable data class PlanStart(
    val afterMs: Long? = null,
    val presets: List<Long> = emptyList(),
    val min: Long = 5_000,
    val max: Long = 840_000,
) {
    companion object {
        fun decode(response: JsonElement): PlanStart = WireJson.decodeFromJsonElement(serializer(), response)
    }
}
