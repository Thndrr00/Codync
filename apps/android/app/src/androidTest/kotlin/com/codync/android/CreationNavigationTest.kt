package com.codync.android

import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createEmptyComposeRule
import androidx.test.core.app.ActivityScenario
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assume.assumeTrue
import org.junit.Rule
import org.junit.Test

/** Opt-in inspection of the real roster route. Never creates a bot or changes accounts or stored data. */
class CreationNavigationTest {
    @get:Rule val compose = createEmptyComposeRule()

    @Test fun newBotMenuOpensNamePurposeAndSuggestions() {
        assumeTrue(InstrumentationRegistry.getArguments().getString("inspectCreation") == "true")
        ActivityScenario.launch(MainActivity::class.java).use {
            compose.waitUntil(30_000) {
                compose.onAllNodesWithContentDescription("New").fetchSemanticsNodes().isNotEmpty()
            }
            compose.onAllNodesWithContentDescription("New").onFirst().performClick()
            compose.onNodeWithText("New bot").performClick()
            try {
                compose.onNode(hasSetTextAction() and hasText("Name your bot")).assertIsDisplayed()
                compose.onNodeWithText("What should this bot help with?").performScrollTo().assertIsDisplayed()
                compose.waitUntil(30_000) {
                    compose.onAllNodesWithText("Suggestions").fetchSemanticsNodes().isNotEmpty()
                }
                compose.onNodeWithText("Suggestions").performScrollTo().assertIsDisplayed()
            } finally {
                // Dismiss the sheet, even on assertion failure; never tap Create.
                compose.onNodeWithContentDescription("Close").performClick()
            }
            compose.onNode(hasSetTextAction() and hasText("Name your bot")).assertDoesNotExist()
        }
    }
}
