import { Observable } from '../lib/observable.ts'
import type { HostClient } from '../client/host-client'

export class VoiceDecisionsStore extends Observable {
  summary = ''
  confirmed: string | null = null
  busy = false
  loading = true
  error: string | null = null
  private client: () => Promise<HostClient>
  private botId: string
  private receiptId: string
  private lifetime = 0
  private active = true
  constructor(client: () => Promise<HostClient>, botId: string, receiptId: string) {
    super(); this.client = client; this.botId = botId; this.receiptId = receiptId
  }
  dispose() { this.active = false; this.lifetime++; this.loading = false }
  setSummary(text: string) { if (!this.busy && this.confirmed === null) { this.summary = text; this.changed() } }
  async load() {
    this.active = true
    if (this.busy) return
    const lifetime = ++this.lifetime
    this.loading = true; this.error = null; this.changed()
    try {
      const result = await (await this.client()).call<{ entry: { data: { voiceDecisions: { summary: string } } } | null }>('voiceDecisions', { botId: this.botId, receiptId: this.receiptId })
      if (lifetime === this.lifetime) this.confirmed = result.entry?.data.voiceDecisions.summary ?? null
    } catch (error) { if (lifetime === this.lifetime) this.error = error instanceof Error ? error.message : String(error) }
    finally { if (lifetime === this.lifetime) { this.loading = false; this.changed() } }
  }
  async confirm() {
    if (!this.active || this.busy || this.loading || this.confirmed !== null || !this.summary.trim()) return
    const lifetime = this.lifetime
    const summary = this.summary.trim()
    let dispatched = false
    this.busy = true; this.error = null; this.changed()
    try {
      const client = await this.client()
      if (!this.active || lifetime !== this.lifetime) return
      dispatched = true
      await client.call('confirmVoiceDecisions', { botId: this.botId, receiptId: this.receiptId, summary })
      if (lifetime === this.lifetime) this.confirmed = summary
    } catch (error) { if (lifetime === this.lifetime) this.error = error instanceof Error ? error.message : String(error) }
    finally {
      this.busy = false
      if (dispatched && lifetime !== this.lifetime) this.error = 'Confirmation outcome unknown. Reopen the transcript to check the saved receipt before retrying.'
      this.changed()
    }
  }
}
