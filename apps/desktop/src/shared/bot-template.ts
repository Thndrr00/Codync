import type { BotDraft } from './models'

/** Swift's `JSONEncoder` with `.prettyPrinted, .sortedKeys, .withoutEscapingSlashes`; nil fields are left out. */
export function encodeTemplate(draft: BotDraft): string {
  // A template carries what the bot is, not what it's allowed to do here (fleet tools).
  const { id: _id, pinned: _pinned, hidden: _hidden, account: _account, fleetManagement: _fleet, ...rest } = draft
  const sorted = Object.fromEntries(
    Object.entries(rest)
      .filter(([, v]) => v !== null && v !== undefined)
      .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)),
  )
  return JSON.stringify(sorted, null, 2).replace(/^(\s*"(?:[^"\\]|\\.)*"): /gm, '$1 : ')
}
