import type { Bot } from '@shared/models'

/**
 * The small line that stays beside a bot's avatar until its first text or step (OpenBot nightly,
 * Natural response style). It is the host's own activity, read from the live mirror — no polling.
 * With the island's Quiet profile on the island shows nothing, but the chat keeps this line, so
 * the person still knows the bot is working on their request.
 */
export function activityLine(bot: Bot | null | undefined): string {
  return bot?.activity || 'Thinking it through…'
}
