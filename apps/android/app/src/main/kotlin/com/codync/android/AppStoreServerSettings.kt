package com.codync.android

import com.codync.android.core.ServerSettings
import com.codync.android.core.WireJson
import kotlinx.serialization.json.decodeFromJsonElement
import kotlinx.serialization.json.encodeToJsonElement
import kotlinx.serialization.json.jsonObject

// This computer's server settings: Personality, the Claude Code setup switch and the remote control policy.

suspend fun AppStore.serverSettings(): ServerSettings =
    WireJson.decodeFromJsonElement(ready().call("serverSettings"))

suspend fun AppStore.setServerSettings(settings: ServerSettings): ServerSettings =
    WireJson.decodeFromJsonElement(ready().call("setServerSettings", WireJson.encodeToJsonElement(settings)).jsonObject.getValue("settings"))
