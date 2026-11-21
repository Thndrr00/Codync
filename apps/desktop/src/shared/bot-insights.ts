// The read-only bdots-inspired insights a host serves for one bot. The host computes every value;
// clients only render it.

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
  model: string
  provider: string | null
  recentWork: string[]
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
