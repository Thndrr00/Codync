package com.codync.android.core

import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.doubleOrNull
import kotlinx.serialization.json.put

// A declarative UI block a bot shows in its chat (card K3, docs/features/declarative-ui.md). The host checks every
// document against Codync's catalogue before it posts it, so this file only shapes what the card draws and what the
// form answers. The desktop app has the same model in `shared/declarative.ts`, the iPhone in `DeclarativeBlock.swift`.

/** One form field of a block, in body order. */
data class BlockField(
    val id: String,
    val label: String,
    /** `text`, `multiline`, `number`, `date`, `time`, `select` or `toggle`. */
    val type: String,
    val required: Boolean = false,
    val placeholder: String? = null,
    /** A select field's choices, as (value, label). */
    val options: List<Pair<String, String>> = emptyList(),
    /** A starting value the agent suggested; the person may change it before sending. */
    val value: JsonElement? = null,
)

/** One body component: only what the catalogue holds. An unknown one is skipped, never guessed at. */
sealed interface BlockComponent {
    data class Heading(val text: String) : BlockComponent
    data class Text(val text: String) : BlockComponent
    /** A thin line between two parts of the card. */
    object Divider : BlockComponent
    data class Callout(val warning: Boolean, val text: String) : BlockComponent
    data class Field(val field: BlockField) : BlockComponent
    data class Table(val columns: List<String>, val rows: List<List<JsonElement>>) : BlockComponent
    data class Chart(val line: Boolean, val labels: List<String>, val values: List<Double>) : BlockComponent
}

/** One button: `primary`, `secondary` or `destructive`. */
data class BlockAction(val id: String, val label: String, val kind: String)

/** A checked block: what the card draws, its fields and its buttons. */
data class DeclarativeBlock(
    val title: String,
    val body: List<BlockComponent>,
    val actions: List<BlockAction>,
) {
    /** Every form field, in body order. */
    val fields: List<BlockField> get() = body.mapNotNull { (it as? BlockComponent.Field)?.field }

    /** The form's first values: what the agent suggested, else over what the card already carried. */
    fun initialValues(sent: JsonObject?): Map<String, JsonElement> {
        val values = mutableMapOf<String, JsonElement>()
        for (field in fields) {
            val value = sent?.get(field.id) ?: field.value ?: continue
            values[field.id] = value
        }
        return values
    }

    /** The answers to send: blanks left out, so the host can require them. */
    fun answers(values: Map<String, JsonElement>): JsonObject = buildJsonObject {
        for (field in fields) {
            val value = values[field.id] ?: continue
            when (field.type) {
                "toggle" -> value.booleanOrNull?.let { put(field.id, it) }
                "number" -> value.doubleOrNull?.let { put(field.id, it) }
                else -> value.contentOrNull?.trim()?.takeIf { it.isNotEmpty() }?.let { put(field.id, it) }
            }
        }
    }

    /** Whether every required field has an answer, so the buttons can stay off until then. */
    fun complete(values: Map<String, JsonElement>): Boolean = fields.all { field ->
        if (!field.required) return@all true
        when (field.type) {
            "toggle" -> true
            "number" -> values[field.id]?.doubleOrNull != null
            else -> !values[field.id]?.contentOrNull.isNullOrBlank()
        }
    }

    companion object {
        /** The block an entry carries, or null when it isn't one (an older card, or one this build can't read). */
        fun decode(value: JsonObject?): DeclarativeBlock? {
            if (value == null) return null
            val version = (value["version"] as? JsonPrimitive)?.doubleOrNull ?: return null
            if (version != 1.0) return null
            val body = (value["body"] as? JsonArray)?.mapNotNull { component(it) }.orEmpty()
            if (body.isEmpty()) return null
            val actions = (value["actions"] as? JsonArray).orEmpty().mapNotNull { action(it) }
            return DeclarativeBlock(text(value, "title").orEmpty(), body, actions)
        }

        private fun component(node: JsonElement): BlockComponent? {
            val object_ = node as? JsonObject ?: return null
            return when (text(object_, "type")) {
                "heading" -> text(object_, "text")?.let { BlockComponent.Heading(it) }
                "text" -> text(object_, "text")?.let { BlockComponent.Text(it) }
                "divider" -> BlockComponent.Divider
                "callout" -> text(object_, "text")?.let { BlockComponent.Callout(text(object_, "tone") == "warning", it) }
                "field" -> (object_["field"] as? JsonObject)?.let { field(it) }?.let { BlockComponent.Field(it) }
                "table" -> table(object_)
                "chart" -> chart(object_)
                // The host's catalogue is versioned with the app: a component this build doesn't know draws nothing.
                else -> null
            }
        }

        private fun field(node: JsonObject): BlockField? {
            val id = text(node, "id")?.takeIf { it.isNotEmpty() } ?: return null
            val options = (node["options"] as? JsonArray).orEmpty().mapNotNull { choice ->
                val option = choice as? JsonObject ?: return@mapNotNull null
                val value = text(option, "value") ?: return@mapNotNull null
                value to (text(option, "label") ?: value)
            }
            return BlockField(
                id = id,
                label = text(node, "label").orEmpty(),
                type = text(node, "type") ?: "text",
                required = (node["required"] as? JsonPrimitive)?.booleanOrNull == true,
                placeholder = text(node, "placeholder"),
                options = options,
                value = node["value"]?.takeUnless { it is JsonNull },
            )
        }

        private fun table(node: JsonObject): BlockComponent? {
            val columns = (node["columns"] as? JsonArray).orEmpty().mapNotNull { (it as? JsonPrimitive)?.contentOrNull }
            if (columns.isEmpty()) return null
            val rows = (node["rows"] as? JsonArray).orEmpty().mapNotNull { row ->
                (row as? JsonArray)?.takeIf { it.size == columns.size }
            }
            return BlockComponent.Table(columns, rows)
        }

        private fun chart(node: JsonObject): BlockComponent? {
            val labels = (node["labels"] as? JsonArray).orEmpty().mapNotNull { (it as? JsonPrimitive)?.contentOrNull }
            val values = (node["values"] as? JsonArray).orEmpty().mapNotNull { (it as? JsonPrimitive)?.doubleOrNull }
            if (labels.isEmpty() || labels.size != values.size) return null
            return BlockComponent.Chart(line = text(node, "chartType") == "line", labels = labels, values = values)
        }

        private fun action(node: JsonElement): BlockAction? {
            val object_ = node as? JsonObject ?: return null
            val id = text(object_, "id") ?: return null
            return BlockAction(id, text(object_, "label").orEmpty(), text(object_, "kind") ?: "primary")
        }

        private fun text(node: JsonObject, key: String): String? = (node[key] as? JsonPrimitive)?.takeIf { it.isString }?.content
    }
}

/** A bar's height as a fraction of the chart, with a small floor so a zero still shows. */
fun chartFraction(value: Double, values: List<Double>): Float =
    (kotlin.math.abs(value) / maxOf(values.maxOfOrNull { kotlin.math.abs(it) } ?: 1.0, 1.0)).coerceAtLeast(0.02).toFloat()

/** A line chart's points on a 0..1 grid, `y` measured from the bottom. */
fun chartPoints(values: List<Double>): List<Pair<Float, Float>> {
    val peak = maxOf(values.maxOfOrNull { kotlin.math.abs(it) } ?: 1.0, 1.0)
    val step = if (values.size > 1) 1f / (values.size - 1) else 0f
    return values.mapIndexed { index, value -> index * step to (kotlin.math.abs(value) / peak).toFloat() }
}
