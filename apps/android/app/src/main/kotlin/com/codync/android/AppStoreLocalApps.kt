package com.codync.android

import com.codync.android.core.LocalApps
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

// The computer-wide ceiling on local mini-apps, from here (iOS `BotStore+LocalApps`).

/** The saved ceiling, the apps this computer holds and the values the computer offers. */
internal suspend fun AppStore.localApps(): LocalApps = LocalApps.decode(hostQuery("localApps", buildJsonObject { }))

/** Saves a new ceiling; the computer throws its reason when the value isn't one of its choices. */
internal suspend fun AppStore.setLocalAppsLimit(limit: Int) {
    hostQuery("setLocalAppsLimit", buildJsonObject { put("limit", limit) })
}
