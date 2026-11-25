// The read-only bdots-inspired insights a host serves for one bot. The host computes every value;
// clients only render it.

export interface InsightTuning {
  headline: string
  tone: string
  count: string
  total: number
  on: number
  off: number
  fast: boolean
  effort: string
  effortIcon: string
  reasoning: boolean
  summary: { key: string; value: string }[]
  options: { key: string; title: string; value: string; icon: string; tone: string }[]
  enabled: { title: string; value: string; icon: string }[]
}

export interface InsightConnectors {
  headline: string
  tone: string
  total: number
  distinct: number
  duplicates: string[]
  chat: boolean
  mail: boolean
  code: boolean
  money: boolean
  summary: { key: string; value: string }[]
  categories: { kind: string; count: number; icon: string; tone: string; percent: number; bar: string }[]
  connectors: { name: string; category: string; icon: string; tone: string; detail: string }[]
}

export interface InsightIdentity {
  name: string
  headline: string
  tone: string
  band: string
  bandIcon: string
  bandTone: string
  section: string
  mode: string
  rank: number
  color: string
  colorIcon: string
  shape: string
  look: string
  visible: boolean
  score: number
  count: string
  summary: { key: string; value: string }[]
  traits: { id: string; label: string; detail: string; icon: string; tone: string; on: boolean; stateIcon: string; stateTone: string }[]
}

export interface InsightResilience {
  headline: string
  label: string
  tone: string
  score: number
  bar: string
  chain: string
  total: number
  hasChain: boolean
  hasOverride: boolean
  customCommands: number
  selfFallbacks: boolean
  switches: { label: string; on: boolean }[]
  summary: { key: string; value: string }[]
  fallbacks: { backend: string; model: string; command: string; label: string; icon: string; tone: string }[]
}

export interface InsightTrust {
  mode: string
  modeLabel: string
  modeIcon: string
  modeTone: string
  modeDetail: string
  score: number
  tone: string
  label: string
  headline: string
  total: number
  countLabel: string
  broad: number
  command: number
  hasDeny: boolean
  hasBroadAllow: boolean
  asksForEverything: boolean
  decisions: { decision: string; count: number }[]
  summary: { key: string; value: string }[]
  rules: { pattern: string; kind: string; decision: string; icon: string; tone: string; broad: boolean }[]
}

export interface InsightLibrary {
  headline: string
  tone: string
  count: string
  total: number
  distinct: number
  duplicates: string[]
  hasScript: boolean
  hasReference: boolean
  hasAsset: boolean
  summary: { key: string; value: string }[]
  kinds: { kind: string; count: number; icon: string; tone: string; percent: number; bar: string }[]
  skills: { name: string; kind: string; icon: string; tone: string; detail: string }[]
}

export interface InsightRoutineSummary {
  count: string
  active: number
  paused: number
  scheduled: number
  activePercent: number
  bar: string
  next: string
  mix: string
  clock: boolean
  event: boolean
  allScheduled: boolean
  headline: string
  tone: string
  summary: { key: string; value: string }[]
  soonest: { id: string; name: string; enabled: boolean; trigger: string; triggerIcon: string; nextIn: string; stateIcon: string; stateTone: string }[]
  pausedList: { id: string; name: string }[]
}

export interface InsightWeek {
  headline: string
  tone: string
  verdict: string
  total: number
  totalLabel: string
  average: number
  peak: { label: string; count: number } | null
  quietDays: number
  activeStreak: number
  quietStreak: number
  aboveAverage: number
  change: number
  perfectAttendance: boolean
  grew: boolean
  trend: { kind: string; label: string; icon: string; tone: string }
  top: { label: string; count: number; bar: string }[]
  quiet: { label: string; count: number; bar: string }[]
  days: { key: string; label: string; count: number }[]
}

export interface InsightProfile {
  headline: string
  enabled: number
  total: number
  missing: number
  coverage: number
  bar: string
  nextUp: { title: string; icon: string }[]
  caps: { id: string; title: string; detail: string; icon: string; tone: string; enabled: boolean; stateIcon: string; stateTone: string }[]
}

export interface InsightHealth {
  score: number
  label: string
  tone: string
  headline: string
  blocking: boolean
  errors: number
  warnings: number
  counts: { severity: string; count: number }[]
  checks: { id: string; title: string; detail: string; severity: string; tone: string; icon: string }[]
}

export interface InsightActivity {
  headline: string
  steps: number
  errors: number
  dominant: string
  span: string
  total: number
  counts: { kind: string; count: number; percent: number }[]
  groups: { kind: string; label: string; icon: string; tone: string; count: number }[]
  items: { text: string; detail: string; status: string; icon: string; tone: string }[]
  days: { day: string; label: string; count: number; width: number }[]
}

export interface InsightRoutine {
  id: string
  name: string
  enabled: boolean
  schedules: string[]
  nextIn: string | null
  nextInMs: number | null
}

export interface BotInsights {
  botId: string
  description: string
  model: string
  provider: string | null
  recentWork: string[]
  activity: InsightActivity
  health: InsightHealth
  profile: InsightProfile
  week: InsightWeek
  routineSummary: InsightRoutineSummary
  library: InsightLibrary
  trust: InsightTrust
  resilience: InsightResilience
  identity: InsightIdentity
  connectors: InsightConnectors
  tuning: InsightTuning
  routines: InsightRoutine[]
  usage: {
    used: number
    usedText: string
    limit: number | null
    resetsAt: number | null
  }
  skills: {
    total: number
    resources: Record<string, number>
  }
}
