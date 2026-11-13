package com.codync.android

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.codync.android.core.ConnectionGuide
import com.codync.android.design.*

/**
 * How to connect an app (the host's `guide`, host/src/market/guide): what to do at the vendor, and
 * its own page to do it on. Nothing is drawn for an app the host sent no guide for.
 */
@Composable internal fun ConnectionGuideCard(guide: ConnectionGuide?, name: String, hostName: String) {
    if (guide == null || guide.steps.isEmpty()) return
    val uri = LocalUriHandler.current
    val footer = if (guide.signingIn) tr("The sign-in stays on {name}’s own page.").replace("{name}", name)
    else tr("Keys are saved on {name} only.").replace("{name}", hostName)
    CardSection("How to connect", footer = footer) {
        row {
            Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                for ((index, step) in guide.steps.withIndex()) {
                    Row(verticalAlignment = Alignment.Top, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text("${index + 1}.", style = Type.footnote, color = Palette.tertiary)
                        Text(tr(step), Modifier.weight(1f, fill = false), style = Type.body, color = Palette.text)
                    }
                }
                guide.url?.let { url ->
                    SecondaryButton(action = { runCatching { uri.openUri(url) } }) {
                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            Symbol("safari", size = 16.dp, tint = Palette.text)
                            Text(tr("Open {name}’s page").replace("{name}", name))
                        }
                    }
                }
            }
        }
    }
}

/** The key note a row carries before anybody adds the app. */
@Composable internal fun KeyNote(text: String) {
    Row(Modifier.padding(top = 1.dp), verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(4.dp)) {
        Symbol("key", size = 10.dp, tint = Palette.tertiary)
        Text(tr(text), style = Type.caption, color = Palette.tertiary, maxLines = 1, overflow = TextOverflow.Ellipsis)
    }
}
