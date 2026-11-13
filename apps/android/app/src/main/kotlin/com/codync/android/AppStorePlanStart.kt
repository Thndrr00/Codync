package com.codync.android

import com.codync.android.core.PlanStart
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

/** When a plan card of a terminal agent starts the plan by itself. */
suspend fun AppStore.planStart(): PlanStart = PlanStart.decode(hostQuery("planStart", buildJsonObject { }))

/** Saves that time; null waits for the person. The host throws its reason when the time is too short. */
suspend fun AppStore.setPlanStart(afterMs: Long?): PlanStart =
    PlanStart.decode(hostQuery("setPlanStart", buildJsonObject { put("afterMs", afterMs?.let(::JsonPrimitive) ?: JsonNull) }))
