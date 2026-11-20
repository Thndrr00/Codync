import type { BotRef } from '@shared/bot-ref'
import type { Bot, Entry } from '@shared/models'

/** What kind of attention an item asks for, and the icon it gets in the list. */
export type NoticeKind = 'question' | 'approval' | 'needsInput' | 'error'

/** One line of the notification center, built only from state the host already sent. */
export interface Notice {
  /** Stable across re-renders so the bell can remember what the person has seen. */
  key: string
  kind: NoticeKind
  /** Source strings; the view translates them with `tr`. */
  title: string
  detail: string
  /** The chat to jump to. Device access approvals have no chat and use `reviewApprovals`. */
  ref: BotRef | null
  createdAt: number
}

/** The shape of the live store the collector needs (easy to supply in tests). */
export interface NoticeInput {
  roster: { ref: BotRef; bot: Bot; entries: Entry[] }[]
  approvals: { id: string; deviceName: string; createdAt?: number | null }[]
}

const refKey = (ref: BotRef) => `${ref.computerId}/${ref.botId}`

/** A short headline for an unanswered approval card, from the tool that asked. */
export function questionHeadline(toolKind?: string): string {
  switch (toolKind) {
    case 'question': return 'Asks you a question'
    case 'plan': return 'Wants your approval of a plan'
    case 'execute': return 'Wants to run a command'
    default: return 'Wants your approval'
  }
}

/**
 * Collects everything that currently needs the person, newest first: unanswered permission
 * cards and member questions in each chat, device access approvals, and bots that need input
 * or failed. Nothing here polls or invents state — it reads the live stores the window already
 * mirrors, so the list follows the host as it changes.
 */
export function collectNotices(input: NoticeInput): Notice[] {
  const notices: Notice[] = []
  for (const { ref, bot, entries } of input.roster) {
    for (const entry of entries) {
      if (entry.kind === 'permission' && entry.data.status === 'pending') {
        notices.push({
          key: `question:${refKey(ref)}:${entry.id}`,
          kind: 'question',
          title: questionHeadline(entry.data.toolKind),
          detail: bot.name,
          ref,
          createdAt: entry.createdAt,
        })
      } else if (entry.data.memberQuestion?.status === 'pending') {
        notices.push({
          key: `question:${refKey(ref)}:${entry.id}`,
          kind: 'question',
          title: 'Asks you a question',
          detail: bot.name,
          ref,
          createdAt: entry.createdAt,
        })
      }
    }
    if (bot.status === 'needsInput' || bot.status === 'error') {
      notices.push({
        key: `bot:${refKey(ref)}:${bot.status}`,
        kind: bot.status === 'error' ? 'error' : 'needsInput',
        title: bot.status === 'error' ? 'A bot ran into an error' : 'A bot needs your response',
        detail: bot.name,
        ref,
        createdAt: bot.startedAt ?? 0,
      })
    }
  }
  for (const approval of input.approvals) {
    notices.push({
      key: `approval:${approval.id}`,
      kind: 'approval',
      title: 'A device asks for access',
      detail: approval.deviceName,
      ref: null,
      createdAt: approval.createdAt ?? 0,
    })
  }
  return notices.sort((a, b) => b.createdAt - a.createdAt)
}
