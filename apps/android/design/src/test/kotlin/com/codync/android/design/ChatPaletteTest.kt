package com.codync.android.design

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatPaletteTest {
    @Test fun elevenPresetsShareTheirNamesWithTheBackgrounds() {
        assertEquals(11, ChatPalette.colours.size)
        assertEquals(11, ChatPalette.backgrounds.size)
        for (colour in ChatPalette.colours) {
            assertNotEquals(colour.light, colour.dark)
            assertTrue(colour.id, ChatPalette.background(colour.id) != null)
        }
        assertEquals(0xFFE35652, ChatPalette.colour("red")!!.light)
        assertNull(ChatPalette.colour("purple"))
    }

    @Test fun cyclingWalksThePresetsThenReturnsToNone() {
        var id: String? = null
        for (expected in ChatPalette.colours.map { it.id } + listOf(null)) {
            id = ChatPalette.next(id, ChatPalette.colours)
            assertEquals(expected, id)
        }
    }

    @Test fun eachThemeKeepsItsOwnChoice() {
        val settings = AppearanceSettings(chatColourLight = "red", chatColourDark = "blue")
        assertEquals("red", settings.chatColour(dark = false))
        assertEquals("blue", settings.chatColour(dark = true))
        val background = AppearanceSettings(chatBackgroundLight = "green")
        assertEquals("green", background.chatBackground(dark = false))
        assertNull(background.chatBackground(dark = true))
    }
}
