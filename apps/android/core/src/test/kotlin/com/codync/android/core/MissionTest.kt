package com.codync.android.core

import kotlinx.serialization.json.decodeFromJsonElement
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class MissionTest {

    @Test
    fun `a mission decodes with its steps`() {
        val raw = WireJson.parseToJsonElement(
            """
            {
              "id": "m1", "botId": "b1", "title": "Ship it", "goal": "by Friday",
              "status": "waiting", "checkpoint": "step one done", "createdAt": 1, "updatedAt": 2,
              "steps": [
                {"id": "s1", "missionId": "m1", "position": 0, "text": "write it", "status": "done", "at": 1},
                {"id": "s2", "missionId": "m1", "position": 1, "text": "review it", "status": "pending", "at": 2}
              ]
            }
            """,
        )
        val mission = WireJson.decodeFromJsonElement<Mission>(raw)
        assertEquals(MissionStatus.WAITING, mission.status)
        assertEquals("step one done", mission.checkpoint)
        assertEquals(listOf("write it", "review it"), mission.steps.map { it.text })
        assertEquals(MissionStepStatus.DONE, mission.steps[0].status)
        assertEquals("·", mission.steps[1].status.mark)
        assertNull(mission.stopReason)
    }

    @Test
    fun `only a final mission counts as returned`() {
        assertEquals(
            listOf(MissionStatus.DONE, MissionStatus.ABANDONED),
            MissionStatus.entries.filter { it.returned },
        )
    }

    @Test
    fun `each status offers the next move the sheet hovers`() {
        assertEquals(MissionStatus.ACTIVE, MissionStatus.PROPOSED.next)
        assertEquals(MissionStatus.ACTIVE, MissionStatus.WAITING.next)
        assertEquals(MissionStatus.ACTIVE, MissionStatus.DONE.next)
        assertEquals(MissionStatus.ACTIVE, MissionStatus.ABANDONED.next)
        assertEquals(MissionStatus.WAITING, MissionStatus.ACTIVE.next)
        assertEquals("Activate", MissionStatus.PROPOSED.nextTitle)
        assertEquals("Pause", MissionStatus.ACTIVE.nextTitle)
        assertTrue(MissionStatus.DONE.returned && MissionStatus.ABANDONED.returned)
    }
}
