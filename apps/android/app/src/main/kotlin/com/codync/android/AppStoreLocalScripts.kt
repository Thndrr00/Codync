package com.codync.android

import com.codync.android.core.ScriptLimit
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

/** The hourly ceiling on requests local scripts send to routines of this computer. */
suspend fun AppStore.scriptLimit(): ScriptLimit = ScriptLimit.decode(hostQuery("scriptLimit", buildJsonObject { }))

/** Saves a new ceiling; the host throws its reason when the value isn't one of the choices. */
suspend fun AppStore.setScriptLimit(perHour: Int): ScriptLimit =
    ScriptLimit.decode(hostQuery("setScriptLimit", buildJsonObject { put("perHour", perHour) }))
