package com.codync.android

import com.codync.android.core.LocalApps
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

/** The computer-wide ceiling on local mini-apps, and how many it holds. */
suspend fun AppStore.localApps(): LocalApps = LocalApps.decode(hostQuery("localApps", buildJsonObject { }))

/** Saves a new ceiling; the host throws its reason when the value isn't one of the choices. */
suspend fun AppStore.setLocalAppsLimit(limit: Int) {
    hostQuery("setLocalAppsLimit", buildJsonObject { put("limit", limit) })
}
