package com.codync.android.core

import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.doubleOrNull
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** A declarative block (card K3): what the card draws, and the form it answers. */
class DeclarativeBlockTest {

    private val wire = WireJson.parseToJsonElement(
        """
        {
          "version": 1,
          "title": "Book the trip",
          "body": [
            {"type": "heading", "text": "Madrid"},
            {"type": "text", "text": "Two nights."},
            {"type": "divider"},
            {"type": "callout", "tone": "warning", "text": "The price may change."},
            {"type": "field", "field": {"id": "nights", "label": "Nights", "type": "number", "required": true, "value": 2}},
            {"type": "field", "field": {"id": "room", "label": "Room", "type": "select", "required": true,
              "options": [{"value": "single", "label": "Single"}, {"value": "double", "label": "Double"}]}},
            {"type": "field", "field": {"id": "late", "label": "Late arrival", "type": "toggle"}},
            {"type": "table", "columns": ["Item", "Price"], "rows": [["Room", 120]]},
            {"type": "chart", "chartType": "bar", "labels": ["Mon", "Tue"], "values": [2, 5]}
          ],
          "actions": [{"id": "approve", "label": "Approve", "kind": "primary"}]
        }
        """.trimIndent()
    ).jsonObject

    @Test
    fun `a block reads its fields in body order and starts from the agent's values`() {
        val block = requireNotNull(DeclarativeBlock.decode(wire))
        assertEquals("Book the trip", block.title)
        assertEquals(listOf("nights", "room", "late"), block.fields.map { it.id })
        val values = block.initialValues(sent = null)
        assertEquals(2.0, values["nights"]?.jsonPrimitive?.doubleOrNull ?: 0.0, 0.0001)
        assertEquals("single", block.fields[1].options.first().first)
        // Required number and select are unfilled until the person answers them.
        assertFalse(block.complete(values))
        assertTrue(block.complete(values + ("room" to JsonPrimitive("double"))))
    }

    @Test
    fun `answers leave blanks out and keep a toggle's no`() {
        val block = requireNotNull(DeclarativeBlock.decode(wire))
        val answers = block.answers(mapOf("nights" to JsonPrimitive(3), "room" to JsonPrimitive("double"), "late" to JsonPrimitive(false)))
        assertEquals(3.0, answers.getValue("nights").jsonPrimitive.doubleOrNull ?: 0.0, 0.0001)
        assertEquals("double", answers.getValue("room").jsonPrimitive.content)
        assertEquals(false, answers.getValue("late").jsonPrimitive.booleanOrNull ?: true)
        // A blank optional field is not sent, so the host can require it if it wants to.
        assertNull(block.answers(mapOf("nights" to JsonPrimitive(3))).get("room"))
    }

    @Test
    fun `an older or unknown block draws nothing instead of half a card`() {
        val other = JsonObject(wire + ("version" to JsonPrimitive(2)))
        assertNull(DeclarativeBlock.decode(other))
        assertNull(DeclarativeBlock.decode(JsonObject(mapOf("version" to JsonPrimitive(1), "body" to kotlinx.serialization.json.JsonArray(emptyList())))))
    }
}
