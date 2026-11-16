package com.codync.android.core

import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement

/** The hourly ceiling on requests local scripts send to routines of this computer (`scriptLimit`), and what it may be. */
@Serializable data class ScriptLimit(val perHour: Int = 30, val choices: List<Int> = emptyList()) {
    companion object {
        fun decode(response: JsonElement): ScriptLimit = WireJson.decodeFromJsonElement(serializer(), response)
    }
}
