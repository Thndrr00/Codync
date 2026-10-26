package com.codync.android

import com.codync.android.core.PendingSend
import java.util.UUID

/**
 * A team run is a job for several bots that one coordinator bot carries out with its `run_pipeline` (stages one
 * after another, each getting the previous reply) or `delegate_batch` (tasks at the same time) tool. The app doesn't
 * run it: it checks the plan and writes the request that goes to the coordinator's chat. The limits are the host's,
 * and the wording is the desktop's (`shared/team-run.ts`).
 */
internal enum class TeamRunMode { Pipeline, Parallel }

internal data class TeamRunStage(val botId: String = "", val goal: String = "")

internal data class TeamRunDraft(
    val mode: TeamRunMode = TeamRunMode.Pipeline,
    /** What the first stage gets as the previous result (pipeline only). */
    val input: String = "",
    val stages: List<TeamRunStage> = listOf(TeamRunStage()),
)

internal const val MAX_TEAM_RUN_STAGES = 8
internal const val MAX_TEAM_RUN_GOAL_BYTES = 8_000

/** Why the plan can't be sent yet, or null when it can. */
internal fun teamRunIssue(draft: TeamRunDraft, coordinatorId: String): String? {
    val stages = draft.stages
    if (coordinatorId.isEmpty()) return "Choose the bot that coordinates the run."
    if (stages.isEmpty()) return "Add at least one stage."
    if (stages.size > MAX_TEAM_RUN_STAGES) return "A run has at most $MAX_TEAM_RUN_STAGES stages."
    val seen = mutableSetOf<String>()
    for ((index, stage) in stages.withIndex()) {
        val n = index + 1
        val goal = stage.goal.trim()
        if (stage.botId.isEmpty()) return "Stage $n needs a bot."
        if (stage.botId == coordinatorId) return "Stage $n: the coordinator can't give itself a task."
        if (goal.isEmpty()) return "Stage $n needs a goal."
        if (goal.toByteArray(Charsets.UTF_8).size > MAX_TEAM_RUN_GOAL_BYTES) return "Stage $n: the goal is too long."
        if (draft.mode == TeamRunMode.Parallel && stage.botId in seen) return "Stage $n: in parallel, each bot gets one task."
        seen += stage.botId
    }
    return null
}

/** The request for the coordinator: names the tool, then lists the stages with their bots' ids. */
internal fun teamRunMessage(draft: TeamRunDraft, botName: (String) -> String): String {
    val pipeline = draft.mode == TeamRunMode.Pipeline
    val lines = mutableListOf(
        if (pipeline) "Run this pipeline with your run_pipeline tool, in this order. Each stage gets the previous stage’s reply; put {{previous}} in its goal where it belongs."
        else "Run these tasks at the same time with your delegate_batch tool. Each bot gets only the task written for it.",
    )
    val input = draft.input.trim()
    if (pipeline && input.isNotEmpty()) { lines += ""; lines += "Input for stage 1 ({{previous}}): $input" }
    lines += ""
    lines += if (pipeline) "Stages:" else "Tasks:"
    draft.stages.forEachIndexed { index, stage -> lines += "${index + 1}. ${botName(stage.botId)} (botId: ${stage.botId}): ${stage.goal.trim()}" }
    lines += ""
    lines += "When it is done, tell me what came back."
    return lines.joinToString("\n")
}

/** Sends [text] to [botId]'s main chat, saved first like any message so it survives a lost connection. */
internal fun AppStore.sendToBot(botId: String, text: String) {
    if (text.isBlank()) return
    val send = PendingSend(UUID.randomUUID().toString(), botId, text.trim(), System.currentTimeMillis())
    action("save:" + send.nonce) { savePending(send); deliver(send) }
}
