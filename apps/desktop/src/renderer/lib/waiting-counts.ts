// The state of a "Waiting for replies" list (OpenBot nightly triage-13388). The header stops
// saying "Waiting" once every request replied or failed and at least one failed. Pure, so the
// count that drives the title and mark can be tested without the store.

export type WaitState = 'asked' | 'stopped' | 'replied' | 'failed'

export interface WaitRow {
  state: WaitState
}

export interface WaitCounts {
  total: number
  asked: number
  stopped: number
  replied: number
  failed: number
  /** Every request ended and one or more failed: show "Requests ended" with both counts. */
  ended: boolean
}

export function waitingCounts(rows: readonly WaitRow[]): WaitCounts {
  let asked = 0
  let stopped = 0
  let replied = 0
  let failed = 0
  for (const row of rows) {
    switch (row.state) {
      case 'asked': asked += 1; break
      case 'stopped': stopped += 1; break
      case 'replied': replied += 1; break
      case 'failed': failed += 1; break
    }
  }
  const total = asked + stopped + replied + failed
  return {
    total,
    asked,
    stopped,
    replied,
    failed,
    ended: total > 0 && failed > 0 && asked === 0 && stopped === 0 && replied + failed === total,
  }
}
