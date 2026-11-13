package com.codync.android.core

import java.time.LocalDate
import java.time.ZoneId
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject

class ServerLibraryTest {
    private fun artifact(id: String, at: Long) =
        ServerLibraryItem("b1", "Bot", false, LibraryItem.ArtifactItem(at, Artifact(id, "b1", id, 1L, updatedAt = at)))

    private fun file(id: String, name: String, at: Long, image: Boolean) =
        ServerLibraryItem("b1", "Bot", image, LibraryItem.FileItem(at, "e-$id", SharedFile(id, name, 1L, "")))

    @Test
    fun the_filter_keeps_images_and_files_apart() {
        val photo = file("photo", "Trip.PNG", 4, image = true)
        val plan = file("plan", "plan.md", 2, image = false)
        val report = artifact("report", 3)
        val items = listOf(photo, report, plan)

        assertEquals(listOf(4L, 3L, 2L), ServerLibraryFilter.All.apply(items).map { it.at })
        assertEquals(listOf(4L), ServerLibraryFilter.Images.apply(items).map { it.at })
        assertEquals(listOf(3L, 2L), ServerLibraryFilter.Files.apply(items).map { it.at }, "an artifact counts as a file")
        assertEquals(emptyList<ServerLibraryItem>(), ServerLibraryFilter.Images.apply(emptyList()))
    }

    @Test
    fun sections_group_by_the_local_day_newest_first() {
        val today = LocalDate.of(2026, 10, 24)
        val zone = ZoneId.of("UTC")
        val day = 24L * 60 * 60 * 1000
        val noon = today.atStartOfDay(zone).toInstant().toEpochMilli() + 12 * 60 * 60 * 1000
        val items = listOf(
            file("old", "old.md", noon - 30 * day, image = false),
            artifact("today", noon),
            file("week", "week.md", noon - 3 * day, image = false),
        )

        val sections = ServerLibraryItem.sections(items, today, zone)
        assertEquals(listOf(LibraryPeriod.Today, LibraryPeriod.ThisWeek, LibraryPeriod.Earlier), sections.map { it.first })
        assertEquals(listOf("artifact/today"), sections.first().second.map { it.item.key })
    }

    @Test
    fun an_item_reads_what_the_host_sends() {
        val value = Json.parseToJsonElement(
            """{"kind":"file","at":5,"entryId":"e1","botId":"b1","botName":"Writer","image":true,
                "file":{"id":"f1","name":"photo.png","size":9,"sha256":"abc"}}""",
        ).jsonObject
        val item = ServerLibraryItem.decode(value)
        assertEquals("b1/file/e1/f1", item?.id)
        assertEquals("Writer", item?.botName)
        assertEquals(true, item?.image)
        assertEquals(5L, item?.at)
    }
}
