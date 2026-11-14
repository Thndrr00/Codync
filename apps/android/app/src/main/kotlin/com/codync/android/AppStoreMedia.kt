package com.codync.android

import com.codync.android.core.MediaAnswer
import com.codync.android.core.WireJson
import kotlinx.serialization.json.decodeFromJsonElement

// The composer's image and video picker: the models a bot's agent makes files with (host/src/media).

/** What this bot's agent can make, for the picker (`mediaModels`). */
suspend fun AppStore.mediaModels(botId: String): MediaAnswer =
    WireJson.decodeFromJsonElement<MediaAnswer>(hostQuery("mediaModels", body("botId" to botId)))
