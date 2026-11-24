// The Schedule view's wire model and calendar arithmetic. Days are `YYYY-MM-DD` strings in the
// chosen IANA zone, so the grid and the host's occurrences line up on any computer's local clock.

export interface ScheduleItem {
  id: string
  name: string
  at: number
  timezone: string | null
  trigger: string
  botId: string
  kind: string
  description?: string | null
  instruction?: string | null
  /** A disabled routine, shown once a day with a paused mark instead of at every trigger (OpenBot 0.77.0). */
  paused?: boolean
}

export interface ScheduleListing {
  items: ScheduleItem[]
}

/** One calendar chip, with the paused flag resolved for the zone the view draws. */
export interface ScheduleRow {
  key: string
  item: ScheduleItem
  paused: boolean
}

/**
 * Turns the host's moments into rows: enabled routines keep every occurrence, while a paused one
 * contributes a single row on each day it would have run. Pure, so the rule is tested on its own.
 */
export function scheduleRows(items: ScheduleItem[], zone: string): ScheduleRow[] {
  const pausedDays = new Set<string>()
  const rows: ScheduleRow[] = []
  for (const item of items) {
    const paused = item.paused === true
    const day = dayOf(item.at, zone)
    if (paused) {
      const key = `${item.id}:${day}`
      if (pausedDays.has(key)) continue
      pausedDays.add(key)
    }
    rows.push({ key: paused ? `paused:${item.id}:${day}` : `${item.id}:${item.at}`, item, paused })
  }
  rows.sort((a, b) => a.item.at - b.item.at || a.key.localeCompare(b.key))
  return rows
}

export type ScheduleMode = 'month' | 'week' | 'day'

/** A day as it appears in `zone`, from an absolute instant. */
export function dayOf(at: number, zone: string): string {
  const parts = new Intl.DateTimeFormat('en-US', {
    timeZone: zone,
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  }).formatToParts(new Date(at))
  const part = (type: string) => parts.find((p) => p.type === type)?.value ?? ''
  return `${part('year')}-${part('month')}-${part('day')}`
}

/** Adds whole calendar days to a `YYYY-MM-DD` string, independent of the computer's clock. */
export function addDays(day: string, delta: number): string {
  const date = new Date(`${day}T00:00:00Z`)
  date.setUTCDate(date.getUTCDate() + delta)
  return date.toISOString().slice(0, 10)
}

/** Monday-based weekday, 0 through 6, for a `YYYY-MM-DD` string. */
export function weekdayOf(day: string): number {
  return (new Date(`${day}T00:00:00Z`).getUTCDay() + 6) % 7
}

export interface ScheduleRange {
  first: string
  last: string
  from: number
  to: number
  days: string[]
}

/** The days a mode draws, plus the absolute range to ask the host for (with a zone margin). */
export function rangeFor(anchor: string, mode: ScheduleMode): ScheduleRange {
  let first = anchor
  let last = anchor
  if (mode === 'week') {
    first = addDays(anchor, -weekdayOf(anchor))
    last = addDays(first, 6)
  } else if (mode === 'month') {
    const monthStart = `${anchor.slice(0, 7)}-01`
    first = addDays(monthStart, -weekdayOf(monthStart))
    last = addDays(first, 41)
  }
  const days: string[] = []
  for (let day = first; day <= last; day = addDays(day, 1)) days.push(day)
  // A day in another zone can straddle UTC midnight; two days either side keeps it in view while
  // staying inside the host's 62-day window (a month grid is 42 days).
  const from = Date.parse(`${first}T00:00:00Z`) - 2 * 86_400_000
  const to = Date.parse(`${last}T00:00:00Z`) + 3 * 86_400_000 - 1
  return { first, last, from, to, days }
}

export function shiftAnchor(anchor: string, mode: ScheduleMode, delta: number): string {
  if (mode === 'day') return addDays(anchor, delta)
  if (mode === 'week') return addDays(anchor, delta * 7)
  const month = new Date(`${anchor.slice(0, 7)}-01T00:00:00Z`)
  month.setUTCMonth(month.getUTCMonth() + delta)
  const year = month.getUTCFullYear()
  const day = Math.min(Number(anchor.slice(8, 10)), new Date(Date.UTC(year, month.getUTCMonth() + 1, 0)).getUTCDate())
  return `${year}-${String(month.getUTCMonth() + 1).padStart(2, '0')}-${String(day).padStart(2, '0')}`
}

export function today(zone: string): string {
  return dayOf(Date.now(), zone)
}

export function rangeLabel(anchor: string, mode: ScheduleMode): string {
  if (mode === 'day') {
    return new Date(`${anchor}T00:00:00Z`).toLocaleDateString(undefined, {
      timeZone: 'UTC',
      weekday: 'long',
      month: 'long',
      day: 'numeric',
    })
  }
  if (mode === 'month') {
    return new Date(`${anchor}T00:00:00Z`).toLocaleDateString(undefined, {
      timeZone: 'UTC',
      month: 'long',
      year: 'numeric',
    })
  }
  const first = addDays(anchor, -weekdayOf(anchor))
  const start = new Date(`${first}T00:00:00Z`).toLocaleDateString(undefined, {
    timeZone: 'UTC',
    month: 'short',
    day: 'numeric',
  })
  const end = new Date(`${addDays(first, 6)}T00:00:00Z`).toLocaleDateString(undefined, {
    timeZone: 'UTC',
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  })
  return `${start} – ${end}`
}

export function timeOf(at: number, zone: string): string {
  return new Date(at).toLocaleTimeString(undefined, { timeZone: zone, hour: '2-digit', minute: '2-digit' })
}

/** The hour 0–23 of an instant in `zone`, without a locale's midnight-as-24 quirk. */
export function hourOf(at: number, zone: string): number {
  const parts = new Intl.DateTimeFormat('en-US', { timeZone: zone, hour: 'numeric', hourCycle: 'h23' }).formatToParts(new Date(at))
  return Number(parts.find((part) => part.type === 'hour')?.value ?? 0)
}

export function dayLabel(day: string): string {
  return new Date(`${day}T00:00:00Z`).toLocaleDateString(undefined, {
    timeZone: 'UTC',
    weekday: 'long',
    month: 'long',
    day: 'numeric',
  })
}
