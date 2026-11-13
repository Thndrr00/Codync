import type { HostClient } from '../client/host-client'
import { Observable } from '../lib/observable.ts'
import type { BotBundle, DoctorReport, TokenBudget } from '@shared/models'

/** Which alerts reach the phone, and the quiet hours around them. */
export interface NotificationRules {
  done: boolean
  needsInput: boolean
  failed: boolean
  paused: boolean
  /** Local hour the quiet window starts (0–23); equal to `quietEnd` means no quiet hours. */
  quietStart: number
  quietEnd: number
  /** The zone the quiet hours are read in; null uses this computer's. */
  timezone: string | null
}

export const DEFAULT_NOTIFICATION_RULES: NotificationRules = {
  done: true, needsInput: true, failed: true, paused: true, quietStart: 0, quietEnd: 0, timezone: null,
}

/** One bot's answer in the comparison. */
export interface ModelCompareResult {
  botId: string
  name: string
  backend: string
  model: string | null
  text?: string
  error?: string
  ms: number
}

/** A message written now that goes out later, as the person's own message in its chat. */
export interface ScheduledMessage {
  id: string
  botId: string
  text: string
  sendAt: number
  createdAt: number
}

/** A page anyone with its link can read, until it expires or is revoked. */
export interface SharedLink {
  token: string
  title: string
  createdAt: number
  expiresAt: number
  bytes: number
  url: string
}

/** The host's upkeep calls the diagnostics page makes: bot check, daily token limits, failed runs, bot bundles. */
export class MaintenanceStore extends Observable {
  report: DoctorReport | null = null
  budgets: TokenBudget[] = []
  retention: { days: number; messages: number } | null = null
  rules: NotificationRules | null = null
  scheduled: ScheduledMessage[] = []
  links: SharedLink[] = []
  backupPath: string | null = null
  compare: ModelCompareResult[] = []
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

  /** How many messages a retention of `days` would delete, without deleting them. */
  async previewRetention(days: number) {
    await this.run(async (c) => {
      this.retention = await c.call<{ days: number; messages: number }>('retentionPreview', { days })
    })
  }

  /** Deletes the messages older than `days`. */
  async pruneRetention(days: number) {
    await this.run(async (c) => {
      const { deleted } = await c.call<{ deleted: number }>('pruneRetention', { days })
      this.retention = null
      this.notice = deleted ? `Deleted ${deleted} old ${deleted === 1 ? 'message' : 'messages'}.` : 'Nothing was older than that.'
    })
  }

  /** The alerts' rules and the messages waiting to go out, for the Diagnostics page. */
  async loadNotifications() {
    await this.run(async (c) => {
      const [rules, scheduled] = await Promise.all([
        c.call<{ rules: NotificationRules }>('notificationRules', {}),
        c.call<{ messages: ScheduledMessage[] }>('scheduledMessages', {}),
      ])
      this.rules = rules.rules
      this.scheduled = scheduled.messages
    })
  }

  /** Saves the alert rules; the host checks them and refuses impossible hours or an unknown zone. */
  async saveRules(rules: NotificationRules) {
    await this.run(async (c) => {
      this.rules = (await c.call<{ rules: NotificationRules }>('setNotificationRules', { rules })).rules
      this.notice = 'Notification rules saved.'
    })
  }

  /** The same prompt through two or three bots, each in a throwaway session, compared side by side. */
  async compareModels(botIds: string[], prompt: string) {
    await this.run(async (c) => {
      const { results } = await c.call<{ results: ModelCompareResult[] }>('compareModels', { botIds, prompt }, 600_000)
      this.compare = results
      this.notice = `Compared ${results.length} ${results.length === 1 ? 'bot' : 'bots'}.`
    })
  }

  /** Changes a waiting message's time, and its text when given. */
  async rescheduleScheduled(id: string, text: string, sendAt: number) {
    await this.run(async (c) => {
      const { rescheduled } = await c.call<{ rescheduled: boolean }>('rescheduleScheduled', { id, text, sendAt })
      if (rescheduled) {
        this.scheduled = (await c.call<{ messages: ScheduledMessage[] }>('scheduledMessages', {})).messages
      }
      this.notice = rescheduled ? 'Scheduled message updated.' : 'That message was already sent.'
    })
  }

  /** Takes a waiting message back before its time. */
  async cancelScheduled(id: string) {
    await this.run(async (c) => {
      const { cancelled } = await c.call<{ cancelled: boolean }>('cancelScheduled', { id })
      if (cancelled) this.scheduled = this.scheduled.filter((message) => message.id !== id)
      this.notice = cancelled ? 'Scheduled message cancelled.' : 'That message was already sent.'
    })
  }

  /** Writes a consistent copy of the database under the host's data folder. */
  async backupDatabase() {
    await this.run(async (c) => {
      const { path } = await c.call<{ path: string }>('backupDatabase', {}, 120_000)
      this.backupPath = path
      this.notice = 'Backup saved.'
    })
  }

  /** The read-only links this computer still serves. */  async loadLinks() {
    await this.run(async (c) => {
      this.links = (await c.call<{ links: SharedLink[] }>('sharedLinks', {})).links
    })
  }

  /** Stops serving a link now; its page is gone for everyone. */
  async revokeLink(token: string) {
    await this.run(async (c) => {
      const { revoked } = await c.call<{ revoked: boolean }>('revokeShare', { token })
      if (revoked) this.links = this.links.filter((link) => link.token !== token)
      this.notice = revoked ? 'Link revoked.' : 'That link was already gone.'
    })
  }

  /** Turns off, for every bot, the skills bots wrote that nobody has used for 30 days. They stay saved. */
  async retireDormantSkills() {
    await this.run(async (c) => {
      const { turnedOff } = await c.call<{ turnedOff: number }>('retireDormantSkills', {})
      this.notice = turnedOff
        ? `Turned off ${turnedOff} unused skill ${turnedOff === 1 ? 'setting' : 'settings'}. The skills stay saved.`
        : 'No unused skills to turn off.'
    })
  }

  /** The bot as bundle text, ready to copy. */
  async exportBot(botId: string): Promise<string | null> {
    let text: string | null = null
    await this.run(async (c) => { text = JSON.stringify(await c.call<BotBundle>('exportBot', { botId }), null, 2) })
    return text
  }

  /** Bots and their documents as one JSON file, copied to the clipboard. */
  async exportWorkspace(): Promise<string | null> {
    let text: string | null = null
    await this.run(async (c) => {
      text = JSON.stringify(await c.call('exportWorkspace', {}), null, 2)
      await window.codync.app.copy(text)
      this.notice = 'Workspace export copied to the clipboard.'
    })
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
