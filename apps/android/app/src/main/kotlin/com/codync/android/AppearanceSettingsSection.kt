package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.codync.android.design.AppearancePreferences
import com.codync.android.design.AppearanceSettings
import com.codync.android.design.ChatPalette
import com.codync.android.design.ChatPreset
import com.codync.android.design.Palette
import com.codync.android.design.SecondaryButton
import com.codync.android.design.Symbol
import com.codync.android.design.Type
import com.kyant.shapes.Capsule
import com.kyant.shapes.RoundedRectangle
import com.codync.android.design.AccentStyle
import com.codync.android.design.AvatarStyle
import com.codync.android.design.tr
import com.codync.android.design.CardSection
import com.codync.android.design.ChatTextScale
import com.codync.android.design.ChoicePicker
import com.codync.android.design.LocalAppearancePreferences
import com.codync.android.design.SurfaceStyle
import com.codync.android.design.ThemeMode
import com.codync.android.design.UiDensity
import com.codync.android.design.ValueRow
import com.codync.android.design.rememberAppearancePreferences
import com.codync.android.design.semibold

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
        row { ChatColourRow(settings, prefs) }
        row { ChatBackgroundRow(settings, prefs) }
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
            ValueRow(tr("Bot avatars"), detail = tr("Plush is shaded and rounded; Flat is even dots.")) {
                ChoicePicker(
                    selection = settings.avatarStyle,
                    options = AvatarStyle.entries.map { it to tr(it.label) },
                    select = prefs::setAvatarStyle,
                    label = tr("Bot avatars"),
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
        row {
            Column(Modifier.fillMaxWidth().padding(vertical = 4.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Text("Live chat preview", style = Type.caption.semibold, color = Palette.secondary)
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                    Text("Review the latest release notes", Modifier.clip(RoundedRectangle(16.dp)).background(Palette.bubbleUser).padding(horizontal = 12.dp, vertical = 8.dp), style = Type.chatBody, color = Palette.text)
                }
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.Start) {
                    Text("All checks passed and the build is ready.", Modifier.clip(RoundedRectangle(16.dp)).background(Palette.bubbleAgent).padding(horizontal = 12.dp, vertical = 8.dp), style = Type.chatBody, color = Palette.text)
                }
            }
        }
        if (settings != AppearanceSettings()) {
            row {
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                    Text("Restore default theme, scale & density", style = Type.caption, color = Palette.secondary)
                    SecondaryButton("Reset appearance") {
                        val def = AppearanceSettings()
                        prefs.setThemeMode(def.themeMode)
                        prefs.setAccentStyle(def.accentStyle)
                        prefs.setBotAccent(def.botAccent)
                        prefs.setSurfaceStyle(def.surfaceStyle)
                        prefs.setChatTextScale(def.chatTextScale)
                        prefs.setDensity(def.density)
                        prefs.setHapticsEnabled(def.hapticsEnabled)
                        prefs.setAvatarStyle(def.avatarStyle)
                        prefs.setChatColour(dark = false, null)
                        prefs.setChatColour(dark = true, null)
                        prefs.setChatBackground(dark = false, null)
                        prefs.setChatBackground(dark = true, null)
                    }
                }
            }
        }
    }
}

/** The person's own bubble colour, one preset per theme: a wrapping grid of dots, the current one ringed. */
@Composable private fun ChatColourRow(settings: AppearanceSettings, prefs: AppearancePreferences) {
    val dark = settings.themeMode.resolvesDark(isSystemInDarkTheme())
    ChatPresetRow(
        title = "Chat colour",
        detail = "The colour of your own messages, for each theme.",
        presets = ChatPalette.colours,
        selected = settings.chatColour(dark),
        dark = dark,
        choose = { prefs.setChatColour(dark, it) },
    )
}

/** The colour behind the chat, one preset per theme. */
@Composable private fun ChatBackgroundRow(settings: AppearanceSettings, prefs: AppearancePreferences) {
    val dark = settings.themeMode.resolvesDark(isSystemInDarkTheme())
    ChatPresetRow(
        title = "Chat background",
        detail = "The colour behind the chat, for each theme.",
        presets = ChatPalette.backgrounds,
        selected = settings.chatBackground(dark),
        dark = dark,
        choose = { prefs.setChatBackground(dark, it) },
    )
}

@Composable private fun ChatPresetRow(
    title: String,
    detail: String,
    presets: List<ChatPreset>,
    selected: String?,
    dark: Boolean,
    choose: (String?) -> Unit,
) {
    Column(Modifier.fillMaxWidth().padding(vertical = 6.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text(tr(title), style = Type.body, color = Palette.text)
            Text(tr(if (dark) "Dark" else "Light"), style = Type.caption, color = Palette.secondary)
        }
        ChoiceGrid {
            Box(Modifier.choice(selected == null, tr("None")) { choose(null) }.ring(selected == null)) {
                Box(Modifier.size(24.dp).clip(Capsule()).background(Palette.bubbleUser), contentAlignment = Alignment.Center) {
                    Symbol("slash", size = 13.dp, tint = Palette.secondary)
                }
            }
            for (preset in presets) {
                Box(Modifier.choice(selected == preset.id, preset.id.replaceFirstChar(Char::uppercase)) { choose(preset.id) }.ring(selected == preset.id)) {
                    Box(Modifier.size(24.dp).clip(Capsule()).background(Color(preset.hex(dark))))
                }
            }
        }
        Text(tr(detail), style = Type.caption, color = Palette.secondary)
    }
}
