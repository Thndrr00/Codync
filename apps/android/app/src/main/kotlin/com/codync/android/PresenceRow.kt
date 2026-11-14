package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.codync.android.core.fill
import com.codync.android.design.*
import kotlinx.coroutines.delay

/** How often the screen asks who else is in the chat; the computer forgets a device thirty seconds
 *  after it last looked, so this also keeps this phone's own place alive while reading. */
private const val POLL = 3_000L

/**
 * Who else is in this chat right now, over the message box: their initials in their colour, and
 * "Ana is typing…" while a key comes from them. Port of OpenBot's team presence line
 * (`TeamPresenceMember`, `TeamPersonAvatar`), with the devices paired to this computer as the
 * people. Nothing shows while this is the only one here.
 */
@Composable fun PresenceRow(botId: String, state: AppState, store: AppStore) {
    LaunchedEffect(botId) {
        while (true) {
            store.presence(botId, if (store.isTyping(botId)) true else null)
            delay(POLL)
        }
    }
    val people = state.present[botId].orEmpty()
    if (people.isEmpty()) return
    val typing = people.filter { it.typing }
    Row(
        Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 2.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Row {
            for (person in people.take(3)) Face(person)
        }
        Text(
            if (typing.isNotEmpty()) typingLine(typing) else hereLine(people),
            style = Type.caption,
            color = Palette.secondary,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}

/** One person's small face: their initials on their colour. */
@Composable private fun Face(person: com.codync.android.core.Person) {
    Box(
        Modifier.size(17.dp).clip(CircleShape).background(AvatarPalette.tint(person.color)),
        contentAlignment = Alignment.Center,
    ) {
        Text(person.initials, style = Type.system(8f).bold, color = Palette.onAccent)
    }
}

/** "Ana is typing…" / "Ana and Bo are typing…" / "Ana, Bo and Cy are typing…". */
@Composable private fun typingLine(people: List<com.codync.android.core.Person>): String {
    val a = people.first()
    val b = people.getOrNull(1)
    return when {
        b == null -> fill(tr("{name} is typing…"), "name" to a.name)
        people.size == 2 -> fill(tr("{a} and {b} are typing…"), "a" to a.name, "b" to b.name)
        else -> fill(tr("{names} are typing…"), "names" to people.joinToString(", ") { it.name })
    }
}

/** "Ana is here" / "Ana and Bo are here" / "Ana, Bo and 2 more are here". */
@Composable private fun hereLine(people: List<com.codync.android.core.Person>): String {
    val a = people.first()
    val b = people.getOrNull(1)
    return when {
        b == null -> fill(tr("{name} is here"), "name" to a.name)
        people.size == 2 -> fill(tr("{a} and {b} are here"), "a" to a.name, "b" to b.name)
        else -> fill(
            tr("{names} and {count} more are here"),
            "names" to people.take(2).joinToString(", ") { it.name },
            "count" to (people.size - 2).toString(),
        )
    }
}
