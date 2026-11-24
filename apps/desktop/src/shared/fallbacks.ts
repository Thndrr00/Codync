import { AUTO_MODE } from './approvals.ts'
import { isGroup, type Bot, type BotDraft, type Fallback, type Resting } from './models.ts'

/** The host refuses more. */
export const MAX_FALLBACKS = 3

/** An agent's rest with the host's additive note that a recent failure has it skipped. */
export type FallbackRest = Resting & { skipReason?: string | null }

/** `agent`'s rest, while it lasts. */
export const restOf = (bot: Bot, agent: string, now: number): FallbackRest | null =>
  bot.resting.find((r) => r.agent === agent && r.until > now) ?? null

/** The agent a bot runs on right now: its fallback's while it's on one. */
export const runningAgent = (bot: Bot) => bot.agentStep?.agent ?? bot.backend

/** The bots run more than one agent, so the roster marks each with its agent's logo. */
export const mixesAgents = (bots: Bot[]) => new Set(bots.filter((b) => !isGroup(b)).map(runningAgent)).size > 1

/** "3:00 PM" today, "Tue 3:00 PM" another day. */
export function clockTime(ms: number, now: number): string {
  const at = new Date(ms)
  const time = at.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' })
  return at.toDateString() === new Date(now).toDateString() ? time : `${at.toLocaleDateString(undefined, { weekday: 'short' })} ${time}`
}

/** A resting agent's second line: "Usage resets 3:00 PM", or "Resting until 3:00 PM". */
export const restLine = (rest: Resting, now: number) =>
  `${rest.reason === 'usageLimit' ? 'Usage resets' : 'Resting until'} ${clockTime(rest.until, now)}`

/** OpenBot 0.66: a backup that failed recently says so instead of showing a count-down. */
export const SKIPPED_NOW = 'Not available now · skipped'

/** The caption under a resting agent: a recent failure is skipped, a usage limit still has a reset. */
export const restCaption = (rest: FallbackRest, now: number): string =>
  rest.skipReason ? SKIPPED_NOW : restLine(rest, now)

/** The Agent card's first line while the bot runs on a fallback; null on its own agent. */
export function stepBanner(bot: Bot, name: (agent: string) => string, now: number): string | null {
  if (!bot.agentStep) return null
  const [to, own] = [name(bot.agentStep.agent), name(bot.backend)]
  const rest = restOf(bot, bot.backend, now)
  return rest && !rest.skipReason
    ? `Using ${to} until ${own} resets at ${clockTime(rest.until, now)}`
    : `Using ${to} while ${own} can't answer`
}

/** The list with the fallback at `index` moved `by` places (clamped to the ends). */
export function moveFallback(list: Fallback[], index: number, by: number): Fallback[] {
  const next = [...list]
  const [moved] = next.splice(index, 1)
  if (moved) next.splice(Math.max(0, Math.min(next.length, index + by)), 0, moved)
  return next
}

/** The bot on agent `backend`: a fallback on that agent goes (an agent is in the chain once, as the host keeps it). */
export const withBackend = (d: BotDraft, backend: string): BotDraft => d.backend === backend ? d : ({
  ...d, backend, model: null, modelOptions: {}, fallbacks: d.fallbacks?.filter((f) => f.backend !== backend),
  // A picked mode becomes the new agent's Auto mode (or, without one, asking).
  mode: d.mode ? AUTO_MODE : d.mode,
})
