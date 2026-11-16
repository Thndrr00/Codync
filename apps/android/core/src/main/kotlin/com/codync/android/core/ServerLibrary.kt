package com.codync.android.core

import java.time.LocalDate
import java.time.ZoneId
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.jsonPrimitive

/** What a server's Library shows (`serverLibrary`): everything, only images, or only other files. */
enum class ServerLibraryFilter(val title: String) {
    All("All"), Images("Images"), Files("Files");

    /** Whether an item shows under this filter; the host applies the same rule to `image`. */
    fun matches(item: ServerLibraryItem): Boolean = when (this) {
        All -> true
        Images -> item.image
        Files -> !item.image
    }

    /** The items this filter keeps, in their order (newest first). */
    fun apply(items: List<ServerLibraryItem>): List<ServerLibraryItem> = items.filter(::matches)
}

/**
 * One thing in a server's Library (`host/src/chat/library.rs`): a chat's item, with the bot that published or shared it,
 * newest first. `image` is true for the extensions the apps preview, as the host marks it.
 */
data class ServerLibraryItem(val botId: String, val botName: String, val image: Boolean, val item: LibraryItem) {
    val at: Long get() = item.at
    val id: String get() = "$botId/${item.key}"

    companion object {
        /** An item, or null for a kind this app doesn't know: the rest of the Library still shows. */
        fun decode(value: JsonObject): ServerLibraryItem? = runCatching {
            ServerLibraryItem(
                botId = requireNotNull(value.text("botId")),
                botName = value.text("botName").orEmpty(),
                image = value["image"]?.jsonPrimitive?.booleanOrNull == true,
                item = requireNotNull(LibraryItem.decode(value)),
            )
        }.getOrNull()

        /** The Library's groups by when each item was published or shared, newest first; empty groups left out. */
        fun sections(
            items: List<ServerLibraryItem>,
            today: LocalDate = LocalDate.now(),
            zone: ZoneId = ZoneId.systemDefault(),
        ): List<Pair<LibraryPeriod, List<ServerLibraryItem>>> =
            items.groupBy { LibraryPeriod.of(it.at, today, zone) }.toList().sortedBy { it.first.ordinal }
    }
}

/** A stable key for an item inside its bot: kind and id, as the host deduplicates them. */
internal val LibraryItem.key: String
    get() = when (this) {
        is LibraryItem.ArtifactItem -> "artifact/${artifact.id}"
        is LibraryItem.FileItem -> "file/$entryId/${file.id}"
        is LibraryItem.AttachmentItem -> "attachment/$entryId/${file.id}"
    }
