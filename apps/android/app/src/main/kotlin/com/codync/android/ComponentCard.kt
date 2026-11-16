package com.codync.android

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.codync.android.core.BlockField
import com.codync.android.core.Entry
import com.codync.android.core.WireJson
import com.codync.android.design.Hairline
import com.codync.android.design.Palette
import com.codync.android.design.PrimaryButton
import com.codync.android.design.SecondaryButton
import com.codync.android.design.Symbol
import com.codync.android.design.Type
import com.codync.android.design.tr
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.doubleOrNull
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

/**
 * One of Codync's own gallery cards (host catalogue version 1, docs/features/component-gallery.md): the
 * host checked it before it was posted, so this maps a catalogue name to Codync's own controls. A name this
 * build doesn't know draws nothing.
 */
@Composable internal fun ComponentCard(entry: Entry, answering: Boolean, respond: (String?, Map<String, JsonElement>?) -> Unit) {
    val card = entry.data["component"] as? JsonObject ?: return
    val name = card.text("name")
    val pending = entry.data.text("status") == "pending"
    val fields = remember(entry.id) { card.formFields() }
    var values by remember(entry.id) { mutableStateOf(valuesFrom(entry.data["values"] as? JsonObject, fields)) }
    val ready = !pending || complete(fields, values)
    val props = card.propsOrNull()
    Column(verticalArrangement = Arrangement.spacedBy(10.dp), modifier = Modifier.padding(top = 4.dp)) {
        Text(card.text("title").ifEmpty { label(name) }, style = Type.body.semibold, color = Palette.text)
        props.optText("caption")?.let {
            Text(it, style = Type.caption, color = Palette.secondary)
        }
        when (name) {
            "metric" -> Metric(props)
            "comparison" -> Grid(props, comparison = true)
            "timeline" -> Timeline(props)
            "table" -> if (props.rowsOrEmpty().isEmpty()) Empty() else Grid(props, comparison = false)
            "checklist" -> if (props.itemsOrEmpty().isEmpty()) Empty() else Checklist(props)
            "form" -> Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                for (field in fields) {
                    FieldRow(field, values[field.id], pending) { id, value -> values = values + (id to value) }
                }
            }
        }
        card.text("source").takeIf { it.isNotEmpty() }?.let {
            Row(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.CenterVertically) {
                Symbol("desktopcomputer", size = 11.dp, tint = Palette.tertiary)
                Text(tr("Filled by the computer"), style = Type.caption2, color = Palette.tertiary)
            }
        }
        if (pending && fields.isNotEmpty()) {
            if (!ready) Text(tr("Fill in the required fields to continue."), style = Type.caption, color = Palette.tertiary)
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                PrimaryButton(props.optText("submit") ?: tr("Send"), enabled = !answering && ready) {
                    respond("submit", answers(fields, values))
                }
                SecondaryButton(tr("Cancel"), enabled = !answering) { respond(null, null) }
            }
        } else if (pending) {
            SecondaryButton(tr("Cancel"), enabled = !answering) { respond(null, null) }
        } else if (entry.data.text("status") == "answered") {
            Text(tr("Answers sent"), style = Type.caption, color = Palette.secondary)
        }
    }
}

@Composable private fun Empty() {
    Text(tr("Nothing to show yet."), style = Type.caption, color = Palette.tertiary)
}

/** A card's name in the person's language. */
private fun label(name: String): String = when (name) {
    "metric" -> tr("Metric")
    "comparison" -> tr("Comparison")
    "timeline" -> tr("Timeline")
    "table" -> tr("Table")
    "checklist" -> tr("Checklist")
    "form" -> tr("Form")
    else -> tr("Card")
}

@Composable private fun Metric(props: JsonObject?) {
    val series = props?.array("series")?.mapNotNull { it.jsonPrimitive.doubleOrNull } ?: emptyList()
    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Text(props?.text("label").orEmpty(), style = Type.caption.medium, color = Palette.secondary)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.Bottom) {
            Text(props?.text("value").orEmpty(), style = Type.title2.semibold, color = Palette.text)
            props.optText("change")?.let { change ->
                val trend = props.text("trend")
                val arrow = when (trend) {
                    "up" -> "\u2191"
                    "down" -> "\u2193"
                    else -> "\u2192"
                }
                Text("$arrow $change", style = Type.footnote, color = if (trend == "down") Palette.attention else Palette.secondary)
            }
        }
        if (series.size > 1) Sparkline(series)
    }
}

/** A quiet line under the number, in the theme's accent. */
@Composable private fun Sparkline(values: List<Double>) {
    val peak = (values.maxOfOrNull { kotlin.math.abs(it) } ?: 1.0).coerceAtLeast(1.0)
    val description = values.joinToString(", ") { it.toString() }
    Canvas(Modifier.fillMaxWidth().height(40.dp).semantics { contentDescription = description }) {
        val path = Path()
        values.forEachIndexed { index, value ->
            val x = if (values.size > 1) size.width * index / (values.size - 1) else 0f
            val y = size.height - size.height * (kotlin.math.abs(value) / peak).toFloat() * 0.9f
            if (index == 0) path.moveTo(x, y) else path.lineTo(x, y)
        }
        drawPath(path, Palette.accent, style = Stroke(width = 1.5f * density))
    }
}

@Composable private fun Grid(props: JsonObject?, comparison: Boolean) {
    val columns = props?.array("columns")?.map { it.jsonPrimitive.contentOrNull.orEmpty() } ?: emptyList()
    val rows = props.rowsOrEmpty()
    Column(Modifier.horizontalScroll(rememberScrollState())) {
        Row(Modifier.padding(bottom = 4.dp)) {
            for (column in columns) {
                Text(column, Modifier.padding(end = 14.dp), style = Type.caption.semibold, color = Palette.secondary)
            }
        }
        Hairline()
        for (row in rows) {
            Row(Modifier.padding(vertical = 4.dp)) {
                row.jsonArray.forEachIndexed { index, cell ->
                    Text(
                        cellText(cell),
                        Modifier.padding(end = 14.dp),
                        style = Type.caption,
                        color = if (comparison && index == 0) Palette.secondary else Palette.text,
                    )
                }
            }
        }
    }
}

@Composable private fun Timeline(props: JsonObject?) {
    val events = props?.array("events") ?: emptyList()
    Column {
        events.forEachIndexed { index, event ->
            val item = event.jsonObject
            Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Canvas(Modifier.size(7.dp).padding(top = 5.dp)) { drawCircle(dot(item.text("tone"))) }
                    if (index < events.size - 1) {
                        Canvas(Modifier.size(width = 1.dp, height = 26.dp)) {
                            drawRect(Palette.border, size = size)
                        }
                    }
                }
                Column(verticalArrangement = Arrangement.spacedBy(2.dp), modifier = Modifier.padding(bottom = 10.dp)) {
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text(item.text("at"), style = Type.caption, color = Palette.tertiary)
                        Text(item.text("title"), style = Type.callout.medium, color = Palette.text)
                    }
                    item.text("detail").takeIf { it.isNotEmpty() }?.let {
                        Text(it, style = Type.footnote, color = Palette.secondary)
                    }
                }
            }
        }
    }
}

private fun dot(tone: String): Color = when (tone) {
    "warning" -> Palette.warning
    "done" -> Palette.added
    else -> Palette.accent
}

@Composable private fun Checklist(props: JsonObject?) {
    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
        for (item in props.itemsOrEmpty()) {
            val row = item.jsonObject
            val done = row["done"]?.jsonPrimitive?.booleanOrNull == true
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.Top) {
                Symbol(if (done) "checkmark.circle.fill" else "circle", size = 13.dp, tint = if (done) Palette.accent else Palette.secondary)
                Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    Text(row.text("label"), style = Type.callout, color = if (done) Palette.secondary else Palette.text)
                    row.text("note").takeIf { it.isNotEmpty() }?.let {
                        Text(it, style = Type.caption, color = Palette.tertiary)
                    }
                }
            }
        }
    }
}

/** A component card's form fields, decoded from the props the host checked. */
private fun JsonObject.formFields(): List<BlockField> =
    propsOrNull()?.array("fields").orEmpty().mapNotNull {
        runCatching { WireJson.decodeFromJsonElement<BlockField>(it) }.getOrNull()
    }

private fun JsonObject.propsOrNull(): JsonObject? = this["props"] as? JsonObject

private fun JsonObject.text(key: String): String = this[key]?.jsonPrimitive?.contentOrNull.orEmpty()

private fun JsonObject?.optText(key: String): String? = this?.text(key)?.takeIf { it.isNotEmpty() }

private fun JsonObject.array(key: String): List<JsonElement>? = (this[key] as? JsonArray)?.toList()

private fun JsonObject?.rowsOrEmpty(): List<JsonElement> = this?.array("rows").orEmpty()

private fun JsonObject?.itemsOrEmpty(): List<JsonElement> = this?.array("items").orEmpty()

private fun cellText(cell: JsonElement): String = when (cell) {
    is JsonNull -> ""
    is JsonPrimitive -> cell.contentOrNull ?: if (cell.booleanOrNull == true) "\u2713" else ""
    is JsonArray -> cell.mapNotNull { it.jsonPrimitive.contentOrNull }.joinToString(" ")
    else -> ""
}

private fun valuesFrom(saved: JsonObject?, fields: List<BlockField>): Map<String, JsonElement> {
    val initial = fields.associate { field ->
        field.id to (if (field.type == "toggle") JsonPrimitive(false) else JsonPrimitive(""))
    }
    return initial + (saved?.entries?.associate { it.key to it.value } ?: emptyMap())
}

private fun complete(fields: List<BlockField>, values: Map<String, JsonElement>): Boolean = fields.all { field ->
    if (!field.required) {
        true
    } else {
        when (field.type) {
            "toggle" -> true
            "number" -> (values[field.id] as? JsonPrimitive)?.doubleOrNull != null
            else -> (values[field.id] as? JsonPrimitive)?.contentOrNull?.trim()?.isNotEmpty() == true
        }
    }
}

/** What the person filled in, by field id; blanks left out so the host can require them. */
private fun answers(fields: List<BlockField>, values: Map<String, JsonElement>): Map<String, JsonElement> {
    val out = mutableMapOf<String, JsonElement>()
    for (field in fields) {
        val value = values[field.id] as? JsonPrimitive
        when (field.type) {
            "number" -> value?.doubleOrNull?.let { out[field.id] = JsonPrimitive(it) }
            "toggle" -> out[field.id] = JsonPrimitive(value?.booleanOrNull == true)
            else -> value?.contentOrNull?.trim()?.takeIf { it.isNotEmpty() }?.let { out[field.id] = JsonPrimitive(it) }
        }
    }
    return out
}
