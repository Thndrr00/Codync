package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.codync.android.design.*
import com.kyant.shapes.RoundedRectangle

/** Where a shared item goes: the user picks a bot and it lands in that bot's draft, never sent for them. */
@Composable internal fun ShareTargetView(state: AppState, share: IncomingShare, store: AppStore) {
    val bots = state.bots.filter { !it.hidden && !it.deleted }
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader("Share to a bot")
        Column(Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(20.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
            Column(Modifier.fillMaxWidth().clip(RoundedRectangle(16.dp)).background(Palette.surface).padding(14.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                if (share.text.isNotBlank()) Text(share.text, style = Type.body, color = Palette.text, maxLines = 4, overflow = TextOverflow.Ellipsis)
                for (file in share.files) IconLabel(file.name, AttachmentIcon.symbol(file.name), Palette.secondary, style = Type.caption)
            }
            Text("It's added to the bot's draft. Nothing is sent until you send it.", style = Type.caption, color = Palette.secondary)
            if (bots.isEmpty()) Text("Connect to a computer with bots to share this.", style = Type.body, color = Palette.secondary)
            else Column(Modifier.fillMaxWidth().clip(RoundedRectangle(16.dp)).background(Palette.surface).padding(horizontal = 14.dp)) {
                for ((index, bot) in bots.withIndex()) {
                    if (index > 0) Hairline()
                    Row(Modifier.fillMaxWidth().plainButton { store.deliverShare(bot.id) }.padding(vertical = 10.dp), verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                        BotAvatar(bot, state.bots, size = 36.dp)
                        Text(bot.name, Modifier.weight(1f), style = Type.body, color = Palette.text, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    }
                }
            }
        }
    }
}
