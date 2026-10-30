package com.codync.android.core

import kotlinx.serialization.json.*
import org.junit.Assert.*
import org.junit.Test

class BotCatalogTest {
    private val wire = """{"bots":[
        {"id":"writer","name":"Writer","summary":"Drafts","instructions":"Write plainly.","avatarColor":"red","avatarShape":"hex","featured":true},
        {"id":"researcher","name":"Research analyst","summary":"Sources","instructions":"Cite.","avatarColor":"cyan","avatarShape":"cloud"}],
      "teams":[{"id":"desk","name":"Research desk","summary":"s","purpose":"p",
        "coordinator":{"id":"lead","name":"Research lead","summary":"","instructions":"Lead.","avatarColor":"cyan","avatarShape":"hex"},
        "members":["researcher","gone","writer"]}]}"""

    @Test fun `the host catalog decodes and a team lists its coordinator first, skipping unknown members`() {
        val catalog = WireJson.decodeFromJsonElement<BotCatalog>(Json.parseToJsonElement(wire))
        assertEquals(listOf("writer"), catalog.bots.filter { it.featured }.map { it.id })
        assertEquals(listOf("lead", "researcher", "writer"), catalog.profiles(catalog.teams.single()).map { it.id })
    }

    @Test fun `a profile fills the look and instructions but keeps the chosen agent and model`() {
        val catalog = WireJson.decodeFromJsonElement<BotCatalog>(Json.parseToJsonElement(wire))
        val draft = BotDraft(backend = "codex", model = "gpt").from(catalog.bots.first())
        assertEquals("Writer", draft.name)
        assertEquals("Write plainly.", draft.description)
        assertEquals("red" to "hex", draft.avatarColor to draft.avatarShape)
        assertEquals("codex" to "gpt", draft.backend to draft.model)
    }
}
