// The read-only bdots-inspired insights a host serves for one bot. The host computes every value;
// clients only render it.

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
