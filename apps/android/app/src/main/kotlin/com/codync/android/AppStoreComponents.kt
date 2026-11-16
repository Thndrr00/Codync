package com.codync.android

import com.codync.android.core.ComponentGallery
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

/** The computer's component gallery: its catalogue, saved cards and each bot's withhold list. */
suspend fun AppStore.components(): ComponentGallery = ComponentGallery.decode(hostQuery("listComponents", buildJsonObject { }))

/** Lets a bot use a component, or withholds it from that bot alone. */
suspend fun AppStore.setComponentAccess(bot: String, component: String, published: Boolean) {
    hostQuery(
        "setComponentAccess",
        buildJsonObject {
            put("bot", bot)
            put("component", component)
            put("published", published)
        },
    )
}

/** Shows a saved card in the gallery, or takes it out. */
suspend fun AppStore.publishComponent(id: String, published: Boolean) {
    hostQuery(
        "publishComponent",
        buildJsonObject {
            put("id", id)
            put("published", published)
        },
    )
}
