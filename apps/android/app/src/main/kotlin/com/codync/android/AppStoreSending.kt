package com.codync.android

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.update
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.put

/** Whether a message sent while a bot works joins its reply, on unless the computer's owner chose Queue. */
internal val AppStore.steerWhileWorking: Boolean
    get() = state.value.hello?.get("steerWhileWorking")?.jsonPrimitive?.booleanOrNull != false

/** Saves that choice for the whole computer: shown at once, and taken back when the computer refuses it. */
internal fun AppStore.setSteerWhileWorking(on: Boolean) = action("sending") {
    val before = steerWhileWorking
    showSteer(on)
    try {
        val answer = hostQuery("setSteerWhileWorking", buildJsonObject { put("enabled", on) })
        showSteer(answer.jsonObject["steerWhileWorking"]?.jsonPrimitive?.booleanOrNull ?: on)
    } catch (error: CancellationException) {
        throw error
    } catch (error: Exception) {
        showSteer(before)
        throw error
    }
}

private fun AppStore.showSteer(on: Boolean) {
    mutableState.update { state ->
        state.copy(hello = state.hello?.let { hello -> JsonObject(hello + ("steerWhileWorking" to JsonPrimitive(on))) })
    }
}
