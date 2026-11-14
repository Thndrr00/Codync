package com.codync.android.core

import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement

/**
 * The computer-wide ceiling on local mini-apps (`localApps`): how many it holds, what it may be,
 * and how many bots made them. Only a new app takes a slot.
 */
@Serializable data class LocalApps(val count: Int = 0, val limit: Int = 100, val choices: List<Int> = emptyList(), val botsWithApps: Int = 0) {
    companion object {
        fun decode(response: JsonElement): LocalApps = WireJson.decodeFromJsonElement(serializer(), response)
    }
}
