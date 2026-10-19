package com.codync.android

import com.codync.android.core.Bot
import com.codync.android.design.AvatarPalette

/** Test-only data: never inserted into the mirror or sent to a host. */
internal fun constellationFixtures(count: Int = 30): List<Bot> {
    val now = System.currentTimeMillis()
    return List(count) { index -> Bot(id = "constellation-$index", name = "Bot ${index + 1}",
        avatarShape = AvatarPalette.shapes[index % AvatarPalette.shapes.size],
        avatarColor = AvatarPalette.colors.keys.toList()[index % AvatarPalette.colors.size],
        status = when (index % 3) { 0 -> "working"; 1 -> "needsInput"; else -> "idle" },
        lastAt = now - index * 3_600_000L, unread = if (index % 4 == 0) index + 1 else 0,
        position = index) }
}
