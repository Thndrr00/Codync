package com.codync.android.design

import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.ui.graphics.Color
import org.junit.Assert.*
import org.junit.Test

class BlockBAppearanceTest {

    @Test fun lightDarkAndOledThemeModesResolveDistinctSurfacesAndContrast() {
        val light = resolvePalette(
            systemDark = true,
            settings = AppearanceSettings(themeMode = ThemeMode.Light, accentStyle = AccentStyle.Orange),
        )
        val dark = resolvePalette(
            systemDark = false,
            settings = AppearanceSettings(themeMode = ThemeMode.Dark, accentStyle = AccentStyle.Orange),
        )
        val oled = resolvePalette(
            systemDark = false,
            settings = AppearanceSettings(themeMode = ThemeMode.OledBlack, accentStyle = AccentStyle.Orange),
        )

        assertFalse(light.isDark)
        assertFalse(light.isOled)
        assertTrue(dark.isDark)
        assertFalse(dark.isOled)
        assertTrue(oled.isDark)
        assertTrue(oled.isOled)
        assertEquals(Color(0xFF000000), oled.background)
        assertNotEquals(dark.background, oled.background)
        assertNotEquals(oled.background, oled.surface)
        assertNotEquals(oled.surface, oled.bubbleAgent)

        // Contrast guarantees across Light, Dark, and OLED Black
        for (palette in listOf(light, dark, oled)) {
            assertTrue("Primary text contrast >= 7.0", contrastRatio(palette.text, palette.background) >= 7.0f)
            assertTrue("Secondary text contrast >= 4.5", contrastRatio(palette.secondary, palette.background) >= 4.5f)
            assertTrue("Accent contrast >= 4.5 on background", contrastRatio(palette.accent, palette.background) >= 4.5f)
            assertTrue("User bubble text contrast >= 4.5", contrastRatio(palette.text, palette.bubbleUser) >= 4.5f)
            assertTrue("Bot bubble text contrast >= 4.5", contrastRatio(palette.text, palette.botMessageBackground) >= 4.5f)
        }
    }

    @Test fun dynamicWallpaperAccentAndAllCuratedAccentsPreserveReadableContrast() {
        val trickyCandidates = listOf(
            Color(0xFFFFF59D), // very light yellow
            Color(0xFF0D1B2A), // very dark navy
            Color(0xFF80CBC4), // pastel teal
            Color(0xFFE57373), // soft red
        )

        for (mode in listOf(ThemeMode.Light, ThemeMode.Dark, ThemeMode.OledBlack)) {
            val dark = mode != ThemeMode.Light
            for (style in AccentStyle.entries) {
                for (candidate in trickyCandidates) {
                    val scheme = if (dark) {
                        darkColorScheme(primary = candidate, background = Color(0xFF101214))
                    } else {
                        lightColorScheme(primary = candidate, background = Color(0xFFFAF9F7))
                    }
                    val palette = resolvePalette(
                        systemDark = dark,
                        settings = AppearanceSettings(themeMode = mode, accentStyle = style),
                        dynamicScheme = scheme,
                    )
                    assertTrue(
                        "Accent contrast failed for mode=$mode style=$style",
                        contrastRatio(palette.accent, palette.background) >= 4.5f,
                    )
                    assertTrue(
                        "OnAccent contrast failed for mode=$mode style=$style",
                        contrastRatio(palette.onAccent, palette.accentFill) >= 4.5f,
                    )
                }
            }
        }
    }

    @Test fun botContextualAccentOverridesChatAccentWithoutChangingSemanticColorsOrWhenDisabled() {
        val baseDark = resolvePalette(
            systemDark = true,
            settings = AppearanceSettings(themeMode = ThemeMode.Dark, accentStyle = AccentStyle.Default, botAccent = true),
            botAvatarColor = null,
        )
        val greenBot = resolvePalette(
            systemDark = true,
            settings = AppearanceSettings(themeMode = ThemeMode.Dark, accentStyle = AccentStyle.Default, botAccent = true),
            botAvatarColor = "green",
        )
        val purpleBot = resolvePalette(
            systemDark = true,
            settings = AppearanceSettings(themeMode = ThemeMode.Dark, accentStyle = AccentStyle.Default, botAccent = true),
            botAvatarColor = "purple",
        )
        val disabledBotAccent = resolvePalette(
            systemDark = true,
            settings = AppearanceSettings(themeMode = ThemeMode.Dark, accentStyle = AccentStyle.Default, botAccent = false),
            botAvatarColor = "green",
        )

        assertNotEquals(baseDark.accent, greenBot.accent)
        assertNotEquals(greenBot.accent, purpleBot.accent)
        assertNotEquals(baseDark.bubbleUser, greenBot.bubbleUser)
        assertEquals(baseDark.accent, disabledBotAccent.accent)
        assertEquals(baseDark.bubbleUser, disabledBotAccent.bubbleUser)
        assertEquals(baseDark.danger, greenBot.danger)
        assertEquals(baseDark.warning, greenBot.warning)

        assertTrue(contrastRatio(greenBot.accent, greenBot.background) >= 4.5f)
        assertTrue(contrastRatio(purpleBot.accent, purpleBot.background) >= 4.5f)
        assertTrue(contrastRatio(greenBot.text, greenBot.bubbleUser) >= 4.5f)
    }

    @Test fun chatTextScaleComposesWithSystemFontScaleAndCapsExtremeCombination() {
        assertEquals(0.85f, ChatTextScale.effectiveMultiplier(1.0f, 0.85f), 0.001f)
        assertEquals(1.00f, ChatTextScale.effectiveMultiplier(1.0f, 1.00f), 0.001f)
        assertEquals(1.15f, ChatTextScale.effectiveMultiplier(1.0f, 1.15f), 0.001f)
        assertEquals(1.30f, ChatTextScale.effectiveMultiplier(1.0f, 1.30f), 0.001f)

        // At 1.8x system accessibility fontScale, combined effective scale is capped at 2.0f so UI never clips
        val multAt180 = ChatTextScale.effectiveMultiplier(1.8f, 1.30f)
        assertEquals(2.0f, multAt180 * 1.8f, 0.01f)
    }

    @Test fun recordingHapticsCapturesConfirmedEventsOnlyWhenEnabled() {
        val haptics = RecordingCodyncHaptics(enabled = true)
        haptics.perform(CodyncHaptic.TabSwitch)
        haptics.perform(CodyncHaptic.MessageSent)
        haptics.perform(CodyncHaptic.Reaction)
        assertEquals(
            listOf(CodyncHaptic.TabSwitch, CodyncHaptic.MessageSent, CodyncHaptic.Reaction),
            haptics.events,
        )

        haptics.enabled = false
        haptics.perform(CodyncHaptic.PermissionAction)
        assertEquals(3, haptics.events.size)
    }

    @Test fun sheetDetentMetricsAdaptToKeyboardAndSettlePredictably() {
        val both = setOf(SheetDetent.Medium, SheetDetent.Expanded)
        assertTrue(SheetMetrics.canUseMedium(both, windowHeightDp = 800f, imeBottomPx = 0))
        assertFalse(SheetMetrics.canUseMedium(both, windowHeightDp = 800f, imeBottomPx = 650))
        assertFalse(SheetMetrics.canUseMedium(both, windowHeightDp = 420f, imeBottomPx = 0))

        val mediumOffset = SheetMetrics.anchorOffsetPx(SheetDetent.Medium, windowHeightPx = 1000f, allowMedium = true)
        assertEquals(360f, mediumOffset, 0.5f)

        assertEquals(
            SheetReleaseOutcome.Dismiss,
            SheetMetrics.resolveRelease(currentOffsetPx = 200f, velocityY = 1800f, windowHeightPx = 1000f, allowMedium = true, currentDetent = SheetDetent.Medium),
        )
        assertEquals(
            SheetReleaseOutcome.SettleExpanded,
            SheetMetrics.resolveRelease(currentOffsetPx = 300f, velocityY = -1200f, windowHeightPx = 1000f, allowMedium = true, currentDetent = SheetDetent.Medium),
        )
        assertEquals(
            SheetReleaseOutcome.SettleMedium,
            SheetMetrics.resolveRelease(currentOffsetPx = 340f, velocityY = 0f, windowHeightPx = 1000f, allowMedium = true, currentDetent = SheetDetent.Medium),
        )
        assertEquals(
            SheetReleaseOutcome.Dismiss,
            SheetMetrics.resolveRelease(currentOffsetPx = 650f, velocityY = 0f, windowHeightPx = 1000f, allowMedium = true, currentDetent = SheetDetent.Medium),
        )
    }
}
