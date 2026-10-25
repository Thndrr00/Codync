import type { HostClient } from '../client/host-client'
import { Observable } from '../lib/observable.ts'

export class ResearchSettingsStore extends Observable {
  configured: boolean | null = null
  loading = false
  busy = false
  error: string | null = null
  notice: string | null = null
  private active = true
  private sequence = 0
  private readonly client: () => Promise<Pick<HostClient, 'call'>>
  constructor(client: () => Promise<Pick<HostClient, 'call'>>) { super(); this.client = client }
  resume() { this.active = true }
  dispose() { this.active = false; ++this.sequence; this.loading = false; this.busy = false }
  async load() {
    if (!this.active || this.loading || this.busy) return
    const sequence = ++this.sequence
    this.loading = true; this.error = null; this.changed()
    try {
      const value = await (await this.client()).call<{ configured: boolean }>('researchStatus', {})
      if (this.active && sequence === this.sequence) this.configured = value.configured
    } catch (error) { if (this.active && sequence === this.sequence) this.error = String(error) }
    finally { if (this.active && sequence === this.sequence) { this.loading = false; this.changed() } }
  }
  async save(key: string) {
    if (!this.active || this.busy) return
    const sequence = ++this.sequence
    this.loading = false; this.busy = true; this.error = null; this.notice = null; this.changed()
    try {
      const value = await (await this.client()).call<{ configured: boolean }>('setResearchKey', { key: key.trim() })
      if (!this.active || sequence !== this.sequence) return
      this.configured = value.configured
      this.notice = value.configured ? 'Key saved on this computer. Its validity is checked when you research.' : 'Research key removed.'
    } catch (error) { if (this.active && sequence === this.sequence) this.error = String(error) }
    finally { if (this.active && sequence === this.sequence) { this.busy = false; this.changed() } }
  }
}
