package com.codync.android.core

import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement

/** One catalogue component the host can draw, with a sample the gallery previews. */
@Serializable
data class CatalogueComponent(
    val name: String,
    val version: Int = 1,
    val summary: String = "",
    /** The data functions this component may have the host fill. */
    val data: List<String> = emptyList(),
    /** Props that pass the host's schema. */
    val sample: JsonElement? = null,
    val available: Boolean = true,
)

/** A card the desktop playground saved; the phone lists and withholds them. */
@Serializable
data class ComponentInstance(
    val id: String,
    val name: String,
    val title: String = "",
    val published: Boolean = false,
    val valid: Boolean = true,
)

/** One bot's say over the gallery: the components it may not use. */
@Serializable
data class ComponentBot(val id: String, val name: String, val kind: String = "agent", val withheld: List<String> = emptyList())

/** The computer's gallery: the catalogue, saved cards and each bot's withhold list. */
@Serializable
data class ComponentGallery(
    val version: Int = 1,
    val components: List<CatalogueComponent> = emptyList(),
    val instances: List<ComponentInstance> = emptyList(),
    val bots: List<ComponentBot> = emptyList(),
) {
    companion object {
        fun decode(response: JsonElement): ComponentGallery = WireJson.decodeFromJsonElement(serializer(), response)
    }
}

/** A card's name in the person's language, for code without a composable context. */
fun componentNameKey(name: String): String = when (name) {
    "metric" -> "Metric"
    "comparison" -> "Comparison"
    "timeline" -> "Timeline"
    "table" -> "Table"
    "checklist" -> "Checklist"
    "form" -> "Form"
    else -> "Card"
}
