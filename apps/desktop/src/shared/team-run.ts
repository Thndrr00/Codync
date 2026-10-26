/**
 * A team run is a job for several bots that one coordinator bot carries out with its `run_pipeline` (stages one
 * after another, each getting the previous reply) or `delegate_batch` (tasks at the same time) tool. The app doesn't
 * run it: it checks the plan and writes the request that goes to the coordinator's chat. The limits are the host's.
 */

export type TeamRunMode = 'pipeline' | 'parallel'

export interface TeamRunStage {
  botId: string
  goal: string
}

export interface TeamRunDraft {
  mode: TeamRunMode
  /** What the first stage gets as the previous result (pipeline only). */
  input: string
  stages: TeamRunStage[]
}

export const MAX_TEAM_RUN_STAGES = 8
export const MAX_TEAM_RUN_GOAL_BYTES = 8_000

const bytes = (text: string) => new TextEncoder().encode(text).length

/** Why the plan can't be sent yet, or null when it can. */
export function teamRunIssue(draft: TeamRunDraft, coordinatorId: string): string | null {
  const { stages } = draft
  if (!coordinatorId) return 'Choose the bot that coordinates the run.'
  if (!stages.length) return 'Add at least one stage.'
  if (stages.length > MAX_TEAM_RUN_STAGES) return `A run has at most ${MAX_TEAM_RUN_STAGES} stages.`
  const seen = new Set<string>()
  for (const [index, stage] of stages.entries()) {
    const n = index + 1
    if (!stage.botId) return `Stage ${n} needs a bot.`
    if (stage.botId === coordinatorId) return `Stage ${n}: the coordinator can't give itself a task.`
    if (!stage.goal.trim()) return `Stage ${n} needs a goal.`
    if (bytes(stage.goal.trim()) > MAX_TEAM_RUN_GOAL_BYTES) return `Stage ${n}: the goal is too long.`
    if (draft.mode === 'parallel' && seen.has(stage.botId)) return `Stage ${n}: in parallel, each bot gets one task.`
    seen.add(stage.botId)
  }
  return null
}

/** The request for the coordinator: names the tool, then lists the stages with their bots' ids. */
export function teamRunMessage(draft: TeamRunDraft, botName: (id: string) => string): string {
  const pipeline = draft.mode === 'pipeline'
  const lines = [
    pipeline
      ? 'Run this pipeline with your run_pipeline tool, in this order. Each stage gets the previous stage’s reply; put {{previous}} in its goal where it belongs.'
      : 'Run these tasks at the same time with your delegate_batch tool. Each bot gets only the task written for it.',
  ]
  const input = draft.input.trim()
  if (pipeline && input) lines.push('', `Input for stage 1 ({{previous}}): ${input}`)
  lines.push('', pipeline ? 'Stages:' : 'Tasks:')
  draft.stages.forEach((stage, index) => {
    lines.push(`${index + 1}. ${botName(stage.botId)} (botId: ${stage.botId}): ${stage.goal.trim()}`)
  })
  lines.push('', 'When it is done, tell me what came back.')
  return lines.join('\n')
}
