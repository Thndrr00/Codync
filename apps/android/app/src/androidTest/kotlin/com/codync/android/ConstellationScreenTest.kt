package com.codync.android

import androidx.compose.foundation.layout.*
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.unit.dp
import androidx.test.platform.app.InstrumentationRegistry
import com.codync.android.design.MenuItem
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test

class ConstellationScreenTest {
    @get:Rule val compose = createComposeRule()

    @Test fun smallRostersKeepEveryBotVisible() {
        val bots = mutableStateOf(constellationFixtures(1))
        compose.setContent { Hosted { BotConstellation(bots.value, {}, { emptyList() }) } }
        compose.onNodeWithContentDescription("Bot 1, Working, 1 unread").assertIsDisplayed()
        compose.runOnIdle { bots.value = constellationFixtures(2) }
        compose.onNodeWithContentDescription("Bot 1, Working, 1 unread").assertIsDisplayed()
        compose.onNodeWithContentDescription("Bot 2, Needs you, 0 unread").assertIsDisplayed()
    }

    @Test fun thirtyBotsSupportChatActionsScrollingAndRemoval() {
        val bots = mutableStateOf(constellationFixtures())
        val opened = mutableStateOf("")
        compose.setContent { Hosted {
            Column(Modifier.fillMaxSize().padding(top = 40.dp)) {
                Text("Demo: 30 bots · ${opened.value}")
                BotConstellation(bots.value, open = { opened.value = "Chat ${it.name}" }, actions = { bot ->
                    listOf(MenuItem("Pin") { opened.value = "Pinned ${bot.name}" },
                        MenuItem("Hide from list") { bots.value = bots.value.filter { it.id != bot.id } })
                })
            }
        } }
        compose.onNodeWithContentDescription("Bot 1, Working, 1 unread").assertIsDisplayed().performClick()
        compose.onNodeWithText("Demo: 30 bots · Chat Bot 1").assertIsDisplayed()
        compose.onNodeWithContentDescription("Bot 2, Needs you, 0 unread").performTouchInput { longClick() }
        compose.onNodeWithText("Hide from list").performClick()
        compose.onNodeWithContentDescription("Bot 2, Needs you, 0 unread").assertDoesNotExist()
        compose.onNodeWithContentDescription("Bot 30, Idle, 0 unread").performScrollTo().assertIsDisplayed().performClick()
        compose.onNodeWithText("Demo: 30 bots · Chat Bot 30").assertIsDisplayed()
        val descriptions = compose.onAllNodes(hasContentDescription("Bot ", substring = true)).fetchSemanticsNodes()
        assertEquals(29, descriptions.size)
        val indices = descriptions.map { it.config[SemanticsProperties.TraversalIndex] }
        assertEquals(indices.sorted(), indices)
    }

    @Test fun selectionSurvivesPreferenceRecreation() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val preferences = RosterPreferences(context)
        val original = preferences.presentation
        try {
            val selection = mutableStateOf(RosterPresentation.List)
            compose.setContent { Hosted { RosterPresentationHeader(constellationFixtures(2), selection.value,
                { selection.value = it; preferences.select(it) }) } }
            compose.onNodeWithText("Constelación").performClick()
            assertEquals(RosterPresentation.Constellation, RosterPreferences(context).presentation)
            compose.onNodeWithText("1 trabajando · 1 te necesita").assertIsDisplayed()
            compose.onNodeWithText("Lista").performClick()
            assertEquals(RosterPresentation.List, RosterPreferences(context).presentation)
        } finally { preferences.select(original) }
    }
}
