package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.codync.android.core.CatalogueComponent
import com.codync.android.core.ComponentGallery
import com.codync.android.core.componentNameKey
import com.codync.android.design.CardForm
import com.codync.android.design.CardSection
import com.codync.android.design.CodyncSwitch
import com.codync.android.design.Hairline
import com.codync.android.design.ModalHeader
import com.codync.android.design.Palette
import com.codync.android.design.Type
import com.codync.android.design.tr
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.contentOrNull

/**
 * A computer's component gallery: the cards Codync can draw in a chat, a preview of each, and a switch per
 * bot to withhold one. Authoring a card is the desktop's playground; a phone reads the catalogue and decides
 * who may use it (docs/features/component-gallery.md).
 */
@Composable internal fun ComponentsSheet(store: AppStore) {
    var gallery by remember { mutableStateOf<ComponentGallery?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    LaunchedEffect(store) {
        gallery = runCatching { store.components() }.getOrElse {
            error = it.message ?: tr("That did not work.")
            null
        }
    }
    fun setAccess(bot: String, component: String, published: Boolean) {
        if (busy) return
        busy = true
        error = null
        scope.launch {
            try {
                store.setComponentAccess(bot, component, published)
                gallery = store.components()
            } catch (e: Exception) {
                error = e.message ?: tr("That did not work.")
            } finally {
                busy = false
            }
        }
    }
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader(tr("Components"))
        CardForm {
            CardSection(footer = tr("The cards Codync can draw in a chat.")) {
                val current = gallery
                if (current == null) {
                    row { Text(tr("Loading…"), style = Type.body, color = Palette.secondary) }
                } else {
                    for (component in current.components) {
                        row { ComponentEntry(component) }
                        if (current.bots.isNotEmpty()) {
                            for (bot in current.bots) {
                                val allowed = bot.withheld.none { it == component.name }
                                row {
                                    CodyncSwitch(allowed, { setAccess(bot.id, component.name, it) }, Modifier.fillMaxWidth(), !busy) {
                                        Text(bot.name, modifier = Modifier.weight(1f), style = Type.callout, color = Palette.text)
                                        Text(
                                            if (allowed) tr("Allowed") else tr("Withheld"),
                                            style = Type.caption,
                                            color = Palette.secondary,
                                        )
                                    }
                                }
                            }
                        }
                        row { Hairline() }
                    }
                    if (current.instances.isNotEmpty()) {
                        row { Text(tr("Saved cards"), style = Type.caption.medium, color = Palette.secondary) }
                        for (saved in current.instances) {
                            row {
                                Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                                    Text(saved.title.ifEmpty { tr(componentNameKey(saved.name)) }, Modifier.weight(1f), style = Type.callout, color = Palette.text)
                                    Text(if (saved.published) tr("In the gallery") else tr("Draft"), style = Type.caption, color = Palette.tertiary)
                                }
                            }
                        }
                    }
                }
            }
            error?.let { message -> row { Text(message, style = Type.footnote, color = Palette.danger) } }
        }
    }
}

/** One card's line: its name, what it is, and the first of its sample's content. */
@Composable private fun ComponentEntry(component: CatalogueComponent) {
    Column(verticalArrangement = Arrangement.spacedBy(2.dp), modifier = Modifier.fillMaxWidth()) {
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
            Text(tr(componentNameKey(component.name)), Modifier.weight(1f), style = Type.callout.medium, color = Palette.text)
            Text("v${component.version}", style = Type.caption2, color = Palette.tertiary)
        }
        Text(tr(component.summary), style = Type.caption, color = Palette.secondary)
        preview(component)?.let { Text(it, style = Type.caption, color = Palette.tertiary) }
        if (component.data.isNotEmpty()) {
            Text(component.data.joinToString(", "), style = Type.caption2, color = Palette.tertiary)
        }
    }
}

/** A compact textual preview of a card's sample: its number, or its first rows or steps. */
private fun preview(component: CatalogueComponent): String? {
    val props = component.sample as? JsonObject ?: return null
    fun text(key: String): String = (props[key] as? JsonPrimitive)?.contentOrNull.orEmpty()
    return when (component.name) {
        "metric" -> listOf(text("label"), text("value")).filter { it.isNotEmpty() }.joinToString(" · ").ifEmpty { null }
        "comparison", "table" -> (props["columns"] as? JsonArray)?.takeIf { it.isNotEmpty() }
            ?.joinToString(" · ") { (it as? JsonPrimitive)?.contentOrNull.orEmpty() }
        "checklist" -> (props["items"] as? JsonArray)?.takeIf { it.isNotEmpty() }?.joinToString(" · ") {
            ((it as? JsonObject)?.get("label") as? JsonPrimitive)?.contentOrNull.orEmpty()
        }
        "timeline" -> (props["events"] as? JsonArray)?.takeIf { it.isNotEmpty() }?.joinToString(" · ") {
            ((it as? JsonObject)?.get("title") as? JsonPrimitive)?.contentOrNull.orEmpty()
        }
        "form" -> (props["fields"] as? JsonArray)?.takeIf { it.isNotEmpty() }?.joinToString(" · ") {
            ((it as? JsonObject)?.get("label") as? JsonPrimitive)?.contentOrNull.orEmpty()
        }
        else -> null
    }
}
