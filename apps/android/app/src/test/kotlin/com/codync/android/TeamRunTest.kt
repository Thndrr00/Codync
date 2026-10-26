package com.codync.android

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class TeamRunTest {
    private val names = mapOf("a" to "Alice", "b" to "Bob", "c" to "Cy")
    private fun name(id: String) = names[id] ?: "Unknown"
    private fun draft(mode: TeamRunMode = TeamRunMode.Pipeline, input: String = "", stages: List<TeamRunStage> =
        listOf(TeamRunStage("b", "Research the topic"), TeamRunStage("c", "Write it up: {{previous}}"))) = TeamRunDraft(mode, input, stages)

    @Test fun aCompletePlanHasNoIssue() {
        assertNull(teamRunIssue(draft(), "a"))
        assertNull(teamRunIssue(draft(TeamRunMode.Parallel), "a"))
    }

    @Test fun thePlanNeedsACoordinatorStagesBotsAndGoals() {
        assertTrue(teamRunIssue(draft(), "")!!.contains("coordinates"))
        assertTrue(teamRunIssue(draft(stages = emptyList()), "a")!!.contains("at least one"))
        assertTrue(teamRunIssue(draft(stages = listOf(TeamRunStage("", "x"))), "a")!!.contains("Stage 1 needs a bot"))
        assertTrue(teamRunIssue(draft(stages = listOf(TeamRunStage("b", "   "))), "a")!!.contains("Stage 1 needs a goal"))
    }

    @Test fun theCoordinatorCannotBeOneOfTheStages() {
        assertTrue(teamRunIssue(draft(stages = listOf(TeamRunStage("a", "x"))), "a")!!.contains("itself"))
    }

    @Test fun aBotCanRepeatInAPipelineButNotInParallel() {
        val twice = listOf(TeamRunStage("b", "First"), TeamRunStage("b", "Second"))
        assertNull(teamRunIssue(draft(stages = twice), "a"))
        assertTrue(teamRunIssue(draft(TeamRunMode.Parallel, stages = twice), "a")!!.contains("one task"))
    }

    @Test fun theHostsLimitsAreEnforcedBeforeSending() {
        val many = List(MAX_TEAM_RUN_STAGES + 1) { TeamRunStage(if (it % 2 == 0) "c" else "b", "x") }
        assertTrue(teamRunIssue(draft(stages = many), "a")!!.contains("at most 8"))
        assertTrue(teamRunIssue(draft(stages = listOf(TeamRunStage("b", "é".repeat(4_001)))), "a")!!.contains("too long"))
        assertNull(teamRunIssue(draft(stages = listOf(TeamRunStage("b", "é".repeat(4_000)))), "a"))
    }

    @Test fun aPipelineRequestNamesTheToolTheOrderTheBotsIdsAndTheInput() {
        val text = teamRunMessage(draft(input = "  rust async  "), ::name)
        assertTrue(text.contains("run_pipeline"))
        assertTrue(text.contains("Input for stage 1 ({{previous}}): rust async"))
        assertTrue(text.indexOf("1. Bob (botId: b): Research the topic") < text.indexOf("2. Cy (botId: c): Write it up: {{previous}}"))
        assertTrue(text.contains("tell me what came back"))
    }

    @Test fun aPipelineWithoutInputLeavesTheInputLineOutAndParallelNamesItsOwnTool() {
        assertFalse(teamRunMessage(draft(), ::name).contains("Input for stage 1"))
        val parallel = teamRunMessage(draft(TeamRunMode.Parallel, "ignored"), ::name)
        assertTrue(parallel.contains("delegate_batch"))
        assertFalse(parallel.contains("Input for stage 1") || parallel.contains("run_pipeline"))
        assertTrue(parallel.contains("Tasks:\n1. Bob"))
        assertEquals(parallel.lines().first(), "Run these tasks at the same time with your delegate_batch tool. Each bot gets only the task written for it.")
    }
}
