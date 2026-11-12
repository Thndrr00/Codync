import type { IslandAction, IslandActivity, IslandChannelEvent, IslandRun } from '../../shared/island-activity'
import { healthLines } from '../../shared/health.ts'
import type { Usage } from '../../shared/models'
import type { FlowCanvas } from '../lib/routine-flow'
import { Observable } from '../lib/observable.ts'
import type { AppModel } from './app-model'
import { parseRef, refKey } from '../../shared/bot-ref.ts'
import { activeCall } from '../views/call/active-call.ts'

const errorText = (error: unknown) => error instanceof Error ? error.message : String(error)

/** Reads existing host reports; execution, validation and scheduling stay on the host. */
export class IslandActivityStore extends Observable {
  runs: IslandRun[] = []
  channels: IslandChannelEvent[] = []
  error: string | null = null
  refreshError: string | null = null
  refreshing = false
  private reading = false
  private generation = 0
  private probe = 0
  private usageOverride: Usage | null = null
  private owner: string | null = null
  private timer: ReturnType<typeof setInterval> | undefined
  private active = true

  private readonly app: AppModel

  constructor(app: AppModel) { super(); this.app = app }

  private adoptOwner(id: string | null) {
    if (this.owner === id) return
    ++this.generation
    ++this.probe
    this.usageOverride = null
    this.runs = []
    this.channels = []
    this.error = null
    this.refreshError = null
    this.reading = false
    this.refreshing = false
    this.owner = id
  }

  start() {
    this.active = true
    const poll = async () => {
      try { if (await window.codync.island.enabled() && this.active) await this.load() }
      catch (error) { if (this.active) { this.error = errorText(error); this.changed() } }
    }
    void poll()
    clearInterval(this.timer)
    this.timer = setInterval(() => void poll(), 15_000)
  }

  dispose() {
    this.active = false
    ++this.generation
    ++this.probe
    this.reading = false
    this.refreshing = false
    clearInterval(this.timer)
  }

  get usage() {
    const current = this.app.local?.usage ?? { providers: [] }
    if (this.owner !== this.app.local?.computer.id) return current
    const newest = (usage: Usage) => Math.max(0, ...usage.providers.map((p) => p.updatedAt))
    return this.usageOverride && newest(this.usageOverride) >= newest(current) ? this.usageOverride : current
  }

  snapshot(): IslandActivity {
    const store = this.app.local
    const online = store?.connection.kind === 'online'
    const owned = this.owner === (store?.computer.id ?? null)
    return {
      online, connection: store?.connectionLabel ?? 'No computer', route: store?.hostRoute ?? 'Unknown',
      refreshing: owned && this.refreshing, refreshError: owned ? this.refreshError : null, error: (owned ? this.error : null) ?? store?.lastError ?? null,
      updatedAt: Math.max(0, ...this.usage.providers.map((p) => p.updatedAt)) || null,
      health: store?.health ? healthLines(store.health) : [],
      runs: this.owner === store?.computer.id ? this.runs : [],
      unread: this.app.roster.filter((item) => item.bot.unread > 0).map((item) => ({ ref: refKey(item.ref), name: item.bot.name, count: item.bot.unread })),
      channels: this.owner === store?.computer.id ? this.channels : [],
      call: activeCall.snapshot,
      // n8n's full report contains a sign-in cookie. Only presentation fields leave this window.
      n8n: store?.n8n ? { state: store.n8n.state, error: store.n8n.error, mcp: store.n8n.mcp } : null,
    }
  }

  async perform(action: IslandAction) {
    switch (action.kind) {
      case 'refreshUsage': return this.refreshUsage()
      case 'refreshActivity': return this.load()
      case 'callMute': if (activeCall.snapshot?.id === action.callId) activeCall.invoke('mute'); return
      case 'callEnd': if (activeCall.snapshot?.id === action.callId) activeCall.invoke('end'); return
      case 'stopBot': {
        const ref = parseRef(action.ref)
        if (ref) this.app.store(ref.computerId)?.stop(ref.botId)
        return
      }
    }
  }

  private async refreshUsage() {
    const store = this.app.local
    this.adoptOwner(store?.computer.id ?? null)
    if (this.refreshing || !this.active) return
    this.refreshError = null
    if (store?.connection.kind !== 'online') {
      this.refreshError = 'Connect this computer to refresh usage.'
      this.changed()
      return
    }
    this.refreshing = true
    const probe = ++this.probe
    this.changed()
    try {
      const usage = await (await store.ready()).call<Usage>('usage', { refresh: true })
      if (!this.active || probe !== this.probe || store !== this.app.local) return
      if (usage.providers.length || !this.usage.providers.length) this.usageOverride = usage
    } catch (error) { if (this.active && probe === this.probe && store === this.app.local) this.refreshError = errorText(error) }
    finally { if (this.active && probe === this.probe) { this.refreshing = false; this.changed() } }
  }

  async load() {
    if (!this.active) return
    const store = this.app.local
    this.adoptOwner(store?.computer.id ?? null)
    if (this.reading) return
    if (store?.connection.kind !== 'online') { this.changed(); return }
    this.reading = true
    const generation = ++this.generation
    const runs: IslandRun[] = []
    const errors: string[] = []
    try {
      const client = await store.ready()
      const bots = this.app.roster.filter((item) => item.store === store && item.bot.kind !== 'group')
      // Bound parallel reads so a large roster never floods the host with canvas queries.
      for (let at = 0; at < bots.length; at += 4) {
        const results = await Promise.allSettled(bots.slice(at, at + 4).map(async (item) => {
          const canvas = await client.call<FlowCanvas>('routineFlowCanvas', { botId: item.bot.id })
          for (const card of canvas.routines) for (const run of card.recentRuns.slice(0, 2)) {
            runs.push({ id: run.id, ref: refKey(item.ref), name: card.routine.name, status: run.status, detail: run.detail ?? '', at: run.createdAt,
              steps: run.id === card.steps[0]?.runId ? card.steps.map((step) => ({ name: store.bots.get(step.botId)?.name || step.botId, status: step.status })) : [] })
          }
        }))
        for (const result of results) if (result.status === 'rejected') errors.push(errorText(result.reason))
        if (generation !== this.generation || store !== this.app.local) return
      }
      let channels: IslandChannelEvent[] = []
      try {
        const events = await client.call<{ activity: { id: string; botId: string; source: string; channel: string; mentioned: boolean; at: number }[] }>('channelActivity', { limit: 30, mentionsOnly: true })
        channels = events.activity.filter((event) => event.mentioned).map((event) => ({ id: event.id, source: event.source, channel: event.channel, at: event.at, ref: `${store.computer.id}/${event.botId}` }))
      } catch (error) { errors.push(errorText(error)) }
      if (generation !== this.generation || store !== this.app.local) return
      this.adoptOwner(store.computer.id)
      this.runs = runs.sort((a, b) => b.at - a.at).slice(0, 30)
      this.channels = channels
      this.error = errors.length ? [...new Set(errors)].join(' · ') : null
    } catch (error) { if (this.active && generation === this.generation && store === this.app.local) this.error = errorText(error) }
    finally { if (generation === this.generation && this.active) { this.reading = false; this.changed() } }
  }
}
