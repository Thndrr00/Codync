package com.codync.android

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.codync.android.core.BlockComponent
import com.codync.android.core.BlockField
import com.codync.android.core.DeclarativeBlock
import com.codync.android.core.Entry
import com.codync.android.core.chartFraction
import com.codync.android.core.chartPoints
import com.codync.android.design.*
import com.kyant.shapes.Capsule
import com.kyant.shapes.RoundedRectangle
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.contentOrNull

/**
 * A declarative UI block the bot shows (card K3, docs/features/declarative-ui.md): the host checked it against
 * Codync's catalogue and posted it as an approval card, so answering it uses the same path as every other approval.
 */
@Composable internal fun DeclarativeCard(entry: Entry, answering: Boolean, respond: (String?, Map<String, JsonElement>?) -> Unit) {
    val block = DeclarativeBlock.decode(entry.data["block"] as? JsonObject) ?: return
    val pending = entry.data.text("status") == "pending"
    var values by remember(entry.id) { mutableStateOf(block.initialValues(entry.data["values"] as? JsonObject)) }
    val ready = pending && block.complete(values)
    fun edit(id: String, value: JsonElement) {
        values = values + (id to value)
    }
    Column(
        Modifier.padding(end = 40.dp).widthIn(max = 420.dp).clip(RoundedRectangle(22.dp)).background(Palette.bubbleAgent).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(block.title, Modifier.weight(1f), style = Type.headline, color = Palette.text)
            if (pending) Box(Modifier.size(7.dp).clip(Capsule()).background(Palette.warning))
        }
        for (component in block.body) BlockRow(component, values, pending) { id, value -> edit(id, value) }
        if (pending && block.fields.any { it.required } && !ready) {
            Text(tr("Fill in the required fields to continue."), style = Type.caption, color = Palette.tertiary)
        }
        if (pending) {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for (action in block.actions) {
                    val send = { respond(action.id, if (block.fields.isEmpty()) null else block.answers(values)) }
                    if (action.kind == "primary") {
                        PrimaryButton(action.label, enabled = !answering && ready, action = send)
                    } else {
                        SecondaryButton(action.label, enabled = !answering && ready, action = send)
                    }
                }
                if (block.actions.none { it.kind == "destructive" }) {
                    SecondaryButton(tr("Cancel"), enabled = !answering) { respond(null, null) }
                }
            }
        } else if (block.actions.isNotEmpty()) {
            Text(tr(if (entry.data.text("status") == "answered") "Answers sent" else "Cancel"), style = Type.caption, color = Palette.secondary)
        }
    }
}

/** One body component: the catalogue's own shapes, in Codync's text and controls. */
@Composable private fun BlockRow(
    component: BlockComponent,
    values: Map<String, JsonElement>,
    pending: Boolean,
    edit: (String, JsonElement) -> Unit,
) {
    when (component) {
        is BlockComponent.Heading -> Text(component.text, style = Type.body.semibold, color = Palette.text)
        is BlockComponent.Text -> Text(component.text, style = Type.callout, color = Palette.secondary)
        BlockComponent.Divider -> Hairline()
        is BlockComponent.Callout -> Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Symbol(if (component.warning) "exclamationmark.triangle.fill" else "info.circle", size = 14.dp,
                tint = if (component.warning) Palette.warning else Palette.accent)
            Text(component.text, Modifier.weight(1f), style = Type.footnote, color = Palette.text)
        }
        is BlockComponent.Field -> FieldRow(component.field, values[component.field.id], pending, edit)
        is BlockComponent.Table -> Table(component)
        is BlockComponent.Chart -> Chart(component)
    }
}

/** A form field: a labelled box for text, a picker for choices, a switch for yes or no. */
@Composable internal fun FieldRow(field: BlockField, value: JsonElement?, pending: Boolean, edit: (String, JsonElement) -> Unit) {
    if (field.type == "toggle") {
        CodyncSwitch(value?.booleanOrNull == true, { edit(field.id, JsonPrimitive(it)) }, Modifier.fillMaxWidth(), pending) {
            Text(field.label, style = Type.callout, color = Palette.text)
        }
        return
    }
    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            Text(field.label, style = Type.footnote.medium, color = Palette.secondary)
            if (field.required) Text("*", style = Type.caption2, color = Palette.tertiary)
        }
        if (field.type == "select") {
            ChoicePicker(value?.contentOrNull ?: field.options.firstOrNull()?.first ?: "", field.options,
                { edit(field.id, JsonPrimitive(it)) }, enabled = pending)
        } else {
            val text = value?.contentOrNull.orEmpty()
            FieldBox(text, { typed ->
                edit(field.id, if (field.type == "number") numberValue(typed) else JsonPrimitive(typed))
            }, field.placeholder ?: field.label, if (field.type == "multiline") 3 else 1, if (field.type == "multiline") 6 else 1,
                fill = Palette.background, capitalize = field.type != "number")
        }
    }
}

/** What a number field holds while it is typed: a number once it reads as one, else the text as typed. */
private fun numberValue(text: String): JsonElement {
    if (text.isBlank()) return JsonNull
    val number = text.trim().toDoubleOrNull()
    return if (number == null) JsonPrimitive(text) else JsonPrimitive(number)
}

/** A table of at most 50 rows: the columns and every cell, scrolling sideways on a narrow phone. */
@Composable private fun Table(component: BlockComponent.Table) {
    Column(Modifier.horizontalScroll(rememberScrollState())) {
        Row(Modifier.padding(bottom = 4.dp)) {
            for (column in component.columns) Text(column, Modifier.padding(end = 14.dp), style = Type.caption.semibold, color = Palette.secondary)
        }
        Hairline()
        for (row in component.rows) Row(Modifier.padding(vertical = 4.dp)) {
            for (cell in row) Text(cellText(cell), Modifier.padding(end = 14.dp), style = Type.caption, color = Palette.text)
        }
    }
}

/** A cell as words: an empty one shows nothing, a true box a checkmark. */
private fun cellText(value: JsonElement): String = when (value) {
    JsonNull -> ""
    is JsonPrimitive -> when {
        value.booleanOrNull != null -> if (value.booleanOrNull == true) "✓" else "—"
        else -> value.contentOrNull ?: value.toString()
    }
    else -> value.toString()
}

/** A bar or line chart of at most 12 points, drawn with the accent, its labels under it. */
@Composable private fun Chart(component: BlockComponent.Chart) {
    val accent = Palette.accent
    val label = (if (component.line) "Line chart: " else "Bar chart: ") + component.labels.joinToString(", ")
    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Canvas(Modifier.fillMaxWidth().height(76.dp).semantics { contentDescription = label }) {
            if (component.line) {
                val points = chartPoints(component.values)
                if (points.size > 1) {
                    val path = Path()
                    for ((index, point) in points.withIndex()) {
                        val x = size.width * point.first
                        val y = size.height - (size.height - 6f) * point.second
                        if (index == 0) path.moveTo(x, y) else path.lineTo(x, y)
                    }
                    drawPath(path, accent, style = Stroke(width = 2.dp.toPx()))
                }
            } else {
                val gap = 6.dp.toPx()
                val width = (size.width - gap * (component.values.size - 1)) / component.values.size
                for ((index, value) in component.values.withIndex()) {
                    val height = size.height * chartFraction(value, component.values)
                    drawRoundRect(accent.copy(alpha = 0.85f), topLeft = Offset(index * (width + gap), size.height - height),
                        size = Size(width, height), cornerRadius = CornerRadius(4.dp.toPx()))
                }
            }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            for (text in component.labels) Text(text, Modifier.weight(1f), style = Type.caption2, color = Palette.tertiary,
                maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
    }
}
