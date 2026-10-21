package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.codync.android.design.*

/**
 * Where a notification or widget of a computer no longer on this phone leads (iOS `ChatScreen`'s
 * empty state): a back button and the reason, in place of the bot's chat.
 */
@Composable internal fun RemovedComputerView(back: () -> Unit, manage: () -> Unit) {
    Column(Modifier.fillMaxSize().background(Palette.background).statusBarsPadding()) {
        ScreenHeader({ BackButton(back) })
        Box(Modifier.weight(1f).padding(horizontal = 32.dp), contentAlignment = Alignment.Center) {
            Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(14.dp)) {
                Symbol("desktopcomputer.trianglebadge.exclamationmark", size = 44.dp, tint = Palette.secondary)
                Text("This computer was removed", Modifier.semantics { heading() }, style = Type.title3.semibold, color = Palette.text, textAlign = TextAlign.Center)
                Text("Its bots and chats are still on that computer. Pair it again to open them here.",
                    style = Type.subheadline, color = Palette.secondary, textAlign = TextAlign.Center)
                SecondaryButton("Computers", action = manage)
            }
        }
    }
}
