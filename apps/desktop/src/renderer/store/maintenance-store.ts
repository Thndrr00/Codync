import type { HostClient } from '../client/host-client'
import { Observable } from '../lib/observable.ts'
import type { BotBundle, DoctorReport, TokenBudget } from '@shared/models'

/** The host's upkeep calls the diagnostics page makes: bot check, daily token limits, failed runs, bot bundles. */
export class MaintenanceStore extends Observable {
  report: DoctorReport | null = null
  budgets: TokenBudget[] = []
  busy = false
  error: string | null = null
  notice: string | null = null
  private readonly client: () => Promise<Pick<HostClient, 'call'>>

  constructor(client: () => Promise<Pick<HostClient, 'call'>>) {
    super()
    this.client = client
  }

  async check() {
    await this.run(async (c) => { this.report = await c.call<DoctorReport>('doctor', {}) })
  }

  async loadBudgets() {
    await this.run(async (c) => { this.budgets = (await c.call<{ budgets: TokenBudget[] }>('tokenBudgets', {})).budgets })
  }

  /** `limit` null takes the limit away. `botId` `*` is the limit for all bots together. */
  async setBudget(botId: string, limit: number | null) {
    await this.run(async (c) => {
      this.budgets = (await c.call<{ budgets: TokenBudget[] }>('setTokenBudget', { botId, limit })).budgets
      this.notice = limit === null ? 'Limit removed.' : 'Limit saved.'
    })
  }

  async clearFailedRuns() {
    await this.run(async (c) => {
      const { cleared } = await c.call<{ cleared: number }>('clearFailedRuns', {})
      this.notice = cleared ? `Cleared ${cleared} failed ${cleared === 1 ? 'run' : 'runs'}.` : 'No failed runs to clear.'
    })
  }

  /** The bot as bundle text, ready to copy. */
  async exportBot(botId: string): Promise<string | null> {
    let text: string | null = null
    await this.run(async (c) => { text = JSON.stringify(await c.call<BotBundle>('exportBot', { botId }), null, 2) })
    return text
  }

  async importBots(bundleText: string, cwd: string) {
    await this.run(async (c) => {
      let bundle: BotBundle
      try { bundle = JSON.parse(bundleText) as BotBundle } catch { throw new Error('That text isn’t a bot bundle.') }
      const { bots } = await c.call<{ bots: unknown[] }>('importBots', { bundle, cwd })
      this.notice = `Imported ${bots.length} ${bots.length === 1 ? 'bot' : 'bots'}.`
    })
  }

  private async run(work: (client: Pick<HostClient, 'call'>) => Promise<void>) {
    if (this.busy) return
    this.busy = true; this.error = null; this.notice = null; this.changed()
    try { await work(await this.client()) } catch (error) { this.error = error instanceof Error ? error.message : String(error) }
    finally { this.busy = false; this.changed() }
  }
}
