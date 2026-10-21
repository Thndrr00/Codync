import type { Bot, Entry } from '@shared/models'
import { Observable } from '../lib/observable.ts'
import type { BotStore } from './bot-store'
import type { RosterItem } from './app-model'

export interface RosterHit { id: string; botId: string; threadId: string | null; text: string; seq: number }
interface SearchPage { results: RosterHit[]; next: number | null }
export type HistoryResults =
  | { kind: 'idle' | 'loading' }
  | { kind: 'error'; message: string }
  | { kind: 'ready'; hits: { hit: RosterHit; item: RosterItem }[]; cursors: Map<BotStore, number | null> }

/** Disposing the screen/account invalidates all outstanding reads, including pagination. */
export class RosterQuery extends Observable {
  result: HistoryResults = { kind: 'idle' }
  preview: Entry[] | null = null
  previewError: string | null = null
  private sequence = 0
  private previewSequence = 0
  private timer: ReturnType<typeof setTimeout> | null = null

  dispose() {
    this.sequence++
    this.previewSequence++
    if (this.timer) clearTimeout(this.timer)
  }

  find(query: string, items: RosterItem[], more = false) {
    const sequence = ++this.sequence
    if (this.timer) clearTimeout(this.timer)
    const previous = more && this.result.kind === 'ready' ? this.result : null
    this.result = { kind: query.trim() ? 'loading' : 'idle' }
    this.changed()
    if (!query.trim()) return
    this.timer = setTimeout(() => void this.search(query, items, sequence, previous), more ? 0 : 300)
  }

  private async search(query: string, items: RosterItem[], sequence: number,
    previous: Extract<HistoryResults, { kind: 'ready' }> | null) {
    try {
      const stores = [...new Set(items.map((item) => item.store))]
      const pages = await Promise.all(stores.map(async (store) => {
        const beforeSeq = previous?.cursors.get(store)
        if (previous && beforeSeq == null) return { store, results: [], next: null }
        const page = await (await store.ready()).call<SearchPage>('searchMessages', {
          query, botIds: items.filter((i) => i.store === store).map((i) => i.bot.id), beforeSeq,
        })
        return { store, ...page }
      }))
      if (sequence !== this.sequence) return
      const hits = pages.flatMap(({ store, results }) => results.flatMap((hit) => {
        const item = items.find((i) => i.store === store && i.bot.id === hit.botId)
        return item ? [{ hit, item }] : []
      }))
      this.result = { kind: 'ready', hits: [...(previous?.hits ?? []), ...hits],
        cursors: new Map(pages.map((page) => [page.store, page.next])) }
    } catch (error) {
      if (sequence !== this.sequence) return
      this.result = { kind: 'error', message: `Search failed: ${error instanceof Error ? error.message : error}` }
    }
    this.changed()
  }

  async showPreview(item: RosterItem | null) {
    const sequence = ++this.previewSequence
    this.preview = null
    this.previewError = null
    this.changed()
    if (!item) return
    try {
      const response = await (await item.store.ready()).call<{ entries: Entry[] }>('rosterPreview', { botId: item.bot.id })
      if (sequence !== this.previewSequence) return
      this.preview = response.entries
    } catch (error) {
      if (sequence !== this.previewSequence) return
      this.previewError = `Preview unavailable: ${error instanceof Error ? error.message : error}`
    }
    this.changed()
  }
}

export async function openRosterHit(item: RosterItem, hit: RosterHit, current: () => boolean = () => true): Promise<boolean> {
  const response = await (await item.store.ready()).call<{ entries: Entry[] }>('messageContext', { botId: hit.botId, entryId: hit.id })
  if (!current()) return false
  item.store.openMessageContext(response.entries, hit)
  return true
}

/** UI-only pending state ends on a host event, a new turn, or an actionable timeout. */
export class RosterStops extends Observable {
  pending = new Set<string>()

  async stop(store: BotStore, bot: Bot) {
    const key = JSON.stringify([store.computer.id, bot.id])
    if (this.pending.has(key)) return
    this.pending.add(key)
    this.changed()
    try {
      const latest = store.bots.get(bot.id)
      if (!latest || !['working', 'needsInput'].includes(latest.status)) throw new Error('This bot is no longer working.')
      await (await store.ready()).call('stop', { botId: bot.id })
      await new Promise<void>((resolve, reject) => {
        const check = () => {
          const current = store.bots.get(bot.id)
          if (!current || !['working', 'needsInput'].includes(current.status) || current.startedAt !== latest.startedAt) finish()
        }
        const finish = (error?: Error) => { clearTimeout(timer); unsubscribe(); error ? reject(error) : resolve() }
        const timer = setTimeout(() => finish(new Error('Stop was requested; confirmation is unavailable. Check the connection.')), 20_000)
        const unsubscribe = store.subscribe(check)
        check()
      })
    } finally {
      this.pending.delete(key)
      this.changed()
    }
  }
}
