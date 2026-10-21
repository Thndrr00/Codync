package com.codync.android.core

import org.junit.Assert.assertEquals
import org.junit.Test

class ModelSearchTest {
    private val default = ModelChoice(null, "Default · Balanced", "Agent choice")
    private val balanced = ModelChoice("model-a", "Balanced", "Everyday tasks")
    private val reasoning = ModelChoice("model-b", "Reasoning", "Complex planning")
    private val choices = listOf(default, balanced, reasoning)

    @Test fun `unmatched search keeps default and selected model with its original label`() {
        assertEquals(listOf(default, balanced), modelSearchChoices(choices, "model-a", "no-match"))
        assertEquals(listOf(default), modelSearchChoices(choices, "", "no-match"))
    }

    @Test fun `search matches name id and description without case or surrounding spaces`() {
        for (query in listOf(" REASONING ", "MODEL-B", "planning")) {
            assertEquals(listOf(default, reasoning), modelSearchChoices(choices, "", query))
        }
    }

    @Test fun `matching selected model appears only once and an empty search preserves order`() {
        assertEquals(listOf(default, reasoning), modelSearchChoices(choices, "model-b", "reasoning"))
        assertEquals(choices, modelSearchChoices(choices, "model-a", "  "))
    }

    @Test fun `saved manual model stays available after the catalog changes`() {
        val catalog = AgentModels(listOf(AgentModel("model-a", "Balanced")))
        val manual = ModelChoice("manual-model", "manual-model")
        assertEquals(listOf(catalog.options("manual-model").first(), manual),
            modelSearchChoices(catalog.options("manual-model"), "manual-model", "no-match"))
        assertEquals(listOf(default, manual), modelSearchChoices(choices, "manual-model", "no-match"))
    }
}
