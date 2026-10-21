package com.codync.android

import androidx.compose.runtime.Composable
import com.codync.android.design.AccentStyle
import com.codync.android.design.CardSection
import com.codync.android.design.ChatTextScale
import com.codync.android.design.ChoicePicker
import com.codync.android.design.LocalAppearancePreferences
import com.codync.android.design.SurfaceStyle
import com.codync.android.design.ThemeMode
import com.codync.android.design.UiDensity
import com.codync.android.design.ValueRow
import com.codync.android.design.rememberAppearancePreferences

/**
 * Device-scoped appearance and ergonomics settings: theme mode, dynamic/curated accent,
 * per-bot chat tint, surface style, chat text scale, row density, and haptic feedback.
 */
@Composable internal fun AppearanceSection() {
    val prefs = LocalAppearancePreferences.current ?: rememberAppearancePreferences()
    val settings = prefs.settings
    CardSection(
        title = "Appearance",
        footer = "Saved on this phone. Chat text size applies on top of Android's system font scale.",
    ) {
        row {
            ValueRow("Theme") {
                ChoicePicker(
                    selection = settings.themeMode,
                    options = ThemeMode.entries.map { it to it.label },
                    select = prefs::setThemeMode,
                    label = "Theme",
                )
            }
        }
        row {
            ValueRow("Accent color") {
                ChoicePicker(
                    selection = settings.accentStyle,
                    options = AccentStyle.available().map { it to it.label },
                    select = prefs::setAccentStyle,
                    label = "Accent color",
                )
            }
        }
        row {
            SwitchRow(
                title = "Bot accent in chat",
                isOn = settings.botAccent,
                onChange = prefs::setBotAccent,
            )
        }
        row {
            ValueRow("Surface style") {
                ChoicePicker(
                    selection = settings.surfaceStyle,
                    options = SurfaceStyle.entries.map { it to it.label },
                    select = prefs::setSurfaceStyle,
                    label = "Surface style",
                )
            }
        }
        row {
            ValueRow("Chat text size") {
                ChoicePicker(
                    selection = settings.chatTextScale,
                    options = ChatTextScale.options,
                    select = prefs::setChatTextScale,
                    label = "Chat text size",
                )
            }
        }
        row {
            ValueRow("Density") {
                ChoicePicker(
                    selection = settings.density,
                    options = UiDensity.entries.map { it to it.label },
                    select = prefs::setDensity,
                    label = "Density",
                )
            }
        }
        row {
            SwitchRow(
                title = "Haptic feedback",
                isOn = settings.hapticsEnabled,
                onChange = prefs::setHapticsEnabled,
            )
        }
    }
}
