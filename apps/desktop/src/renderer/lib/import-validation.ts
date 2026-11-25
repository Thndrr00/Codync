/**
 * Why the bot import window cannot import yet (OpenBot 0.73.0: keep the window open with the exact
 * reason). The sheet never closes on a validation failure; the person can change the selection there.
 */
export interface ImportSelection {
  /** Bot keys the export has. */
  bots: readonly string[]
  /** Bot keys the person left checked. */
  picked: readonly string[]
  busy: boolean
}

export function importValidation(input: ImportSelection): string | null {
  if (input.busy) return 'Importing… keep this window open.'
  if (!input.bots.length) return 'This file has no bots to import.'
  if (!input.bots.some((key) => input.picked.includes(key))) return 'Pick at least one bot to import.'
  return null
}
