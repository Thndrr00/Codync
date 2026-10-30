import { Observable } from '../lib/observable.ts'
import type { HostClient } from '../client/host-client'

export type SearchScope = 'all' | 'main' | 'threads'
export interface SearchShortcut { id: string; query: string; scope: SearchScope }
interface SearchResponse { searches: SearchShortcut[] }

/** The host owns validation and durable preferences; this store only mirrors one computer. */
export class SearchShortcuts extends Observable {
  searches: SearchShortcut[] = []
  loading = false
  busy = false
  available = false
  error: string | null = null
  private sequence = 0
  private active = true
  private readonly ready: () => Promise<Pick<HostClient, 'call'>>
  constructor(ready: () => Promise<Pick<HostClient, 'call'>>) { super(); this.ready = ready }
  dispose() { this.active = false; ++this.sequence; this.loading = false; this.busy = false }
  resume() { this.active = true }
  async load() { await this.request('savedSearches', {}, false) }
  async save(query: string, scope: SearchScope) { await this.request('saveSearch', { query, scope }, true) }
  async remove(id: string) { await this.request('deleteSavedSearch', { id }, true) }

  private async request(method: string, body: Record<string, string>, mutation: boolean) {
    if (!this.active || this.loading || this.busy) return
    const sequence = ++this.sequence
    this.loading = !mutation; this.busy = mutation; this.error = null; this.changed()
    try {
      const response = await (await this.ready()).call<SearchResponse>(method, body)
      if (!this.active || sequence !== this.sequence) return
      this.searches = response.searches
      this.available = true
    } catch (error) {
      if (this.active && sequence === this.sequence) this.error = error instanceof Error ? error.message : String(error)
    } finally {
      if (this.active && sequence === this.sequence) { this.loading = false; this.busy = false; this.changed() }
    }
  }
}
