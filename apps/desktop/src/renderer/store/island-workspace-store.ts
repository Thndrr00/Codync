import type { IslandAction } from '../../shared/island-activity'
import type { IslandWorkspace } from '../../shared/island-workspace'
import type { LibraryItem } from '../../shared/artifacts'
import type { Entry, WeekSummary } from '../../shared/models'
import { parseRef } from '../../shared/bot-ref.ts'
import { Observable } from '../lib/observable.ts'
import type { AppModel } from './app-model'
import { islandThreadNavigation } from './island-thread-navigation.ts'

const message = (error: unknown) => error instanceof Error ? error.message : String(error)
const empty = (ref: string): IslandWorkspace => ({ ref, loading: false, busy: false, error: null, actionError: null, artifacts: [], files: [], changes: [], queue: [], git: null, timings: [], day: null })

/** Reads host resources on request. A saved draft is transferred only after an explicit resume intent. */
export class IslandWorkspaceStore extends Observable {
  private value: IslandWorkspace | undefined
  private owner: object | undefined
  private generation = 0
  private alive = true
  private mutating = false
  private entries: Entry[] = []
  private library: LibraryItem[] = []
  private readonly app: AppModel
  private readonly didOpenResource: (ref: string) => void
  constructor(app: AppModel, didOpenResource = (ref: string) => window.codync.island.reveal(ref)) { super(); this.app = app; this.didOpenResource = didOpenResource }

  snapshot() {
    const ref = this.value && parseRef(this.value.ref)
    return ref && this.app.store(ref.computerId) === this.owner ? this.value : undefined
  }

  dispose() { this.alive = false; ++this.generation }
  refresh() { const ref = this.snapshot()?.ref; if (ref && !this.value?.loading) void this.load(ref) }

  async load(key: string) {
    const ref = parseRef(key)
    const store = ref && this.app.store(ref.computerId)
    if (!ref || !store || !store.bots.has(ref.botId) || !this.alive) return
    const generation = ++this.generation
    if (this.value?.ref !== key || this.owner !== store) { this.entries = []; this.library = []; this.value = empty(key) }
    this.owner = store
    this.value = { ...this.value!, computerName: store.computer?.name, loading: true, error: null, busy: this.mutating }
    this.changed()
    const current = () => this.alive && generation === this.generation && this.app.store(ref.computerId) === store
    try {
      if (store.connection.kind !== 'online') throw new Error('Connect this computer to read its workspace.')
      const client = await store.ready()
      if (!current()) return
      const results = await Promise.allSettled([
        client.library(ref.botId),
        client.call<{ entries: Entry[] }>('history', { botId: ref.botId, limit: 100 }),
        client.call<{ entries: IslandWorkspace['queue'] }>('queuedMessages', { botId: ref.botId }),
        client.call<Pick<IslandWorkspace, 'git' | 'timings'>>('islandWorkspace', { botId: ref.botId }),
        client.call<WeekSummary>('weekSummary', { days: 1 }),
      ] as const)
      if (!current()) return
      const [library, history, queue, workspace, day] = results
      this.library = library.status === 'fulfilled' ? library.value.items : []
      this.entries = history.status === 'fulfilled' ? history.value.entries : []
      this.value = {
        ...this.value!, loading: false,
        artifacts: this.library.flatMap((item) => item.kind === 'artifact' ? [{ id: item.artifact.id, title: item.artifact.title, at: item.at }] : []).slice(0, 10),
        files: this.library.flatMap((item) => item.kind === 'file' ? [{ id: item.file.id, entryId: item.entryId, name: item.file.name, at: item.at }] : []).slice(0, 10),
        changes: this.entries.filter((entry) => entry.kind === 'tool' && entry.data.toolKind === 'edit' && entry.data.status === 'completed' && entry.data.diffs?.length).sort((a, b) => b.createdAt - a.createdAt).slice(0, 5).map((entry) => ({ entryId: entry.id, at: entry.createdAt, diffs: entry.data.diffs!.slice(0, 3).map((diff) => ({ ...diff, patch: diff.patch.slice(0, 4000) })) })),
        queue: queue.status === 'fulfilled' ? queue.value.entries : [],
        git: workspace.status === 'fulfilled' ? workspace.value.git : null,
        timings: workspace.status === 'fulfilled' ? workspace.value.timings : [],
        day: day.status === 'fulfilled' ? day.value : null,
        error: results.flatMap((result) => result.status === 'rejected' ? [message(result.reason)] : []).join(' · ') || null,
      }
    } catch (error) {
      if (current()) this.value = { ...empty(key), actionError: this.value?.actionError ?? null, busy: this.mutating, error: message(error) }
    } finally { if (current()) { this.value = { ...this.value!, loading: false }; this.changed() } }
  }

  async perform(action: IslandAction) {
    if (action.kind === 'loadWorkspace') return this.load(action.ref)
    if (!['retryRun', 'cancelQueued', 'openArtifact', 'openFile', 'resumeDraft', 'sendSavedDraft'].includes(action.kind) || !('ref' in action)) return
    const ref = parseRef(action.ref)
    const store = ref && this.app.store(ref.computerId)
    if (!ref || !store || !store.bots.has(ref.botId) || !this.alive) return
    if (action.kind === 'resumeDraft') {
      const text = store.composerDrafts.get(ref.botId)
      if (text.trim()) window.codync.island.offerDraft({ ref: action.ref, text })
      return
    }
    if (action.kind === 'sendSavedDraft') {
      store.send(action.text, ref.botId)
      if (store.composerDrafts.get(ref.botId) === action.original) store.composerDrafts.set(ref.botId, '')
      return
    }
    // Resource navigation is scoped to the most recently requested, still-owned snapshot.
    if (action.kind === 'openArtifact' || action.kind === 'openFile') {
      if (this.snapshot()?.ref !== action.ref) return
      if (action.kind === 'openArtifact') {
        if (!this.library.some((item) => item.kind === 'artifact' && item.artifact.id === action.id)) return
        islandThreadNavigation.open({ kind: 'artifact', ref: action.ref, artifactId: action.id })
      } else {
        const item = this.library.find((item) => item.kind === 'file' && item.entryId === action.entryId && item.file.id === action.id)
        if (item?.kind !== 'file') return
        // Fetch the specific published entry through the existing store, not an arbitrary path.
        const entry = this.entries.find((entry) => entry.id === item.entryId) ?? store.allEntries(ref.botId).find((entry) => entry.id === item.entryId)
        if (!entry) { this.fail('Open the conversation to load this older file.'); return }
        islandThreadNavigation.open({ kind: 'file', ref: action.ref, file: item.file, entryId: item.entryId, entry })
      }
      this.app.select(ref)
      this.didOpenResource(action.ref)
      return
    }
    if (action.kind !== 'retryRun' && action.kind !== 'cancelQueued') return
    if (this.mutating) return
    if (this.value?.ref !== action.ref || this.owner !== store) { this.value = empty(action.ref); this.owner = store; ++this.generation }
    this.mutating = true
    if (this.value) this.value = { ...this.value, busy: true, actionError: null }
    this.changed()
    try {
      const client = await store.ready()
      if (!this.alive || this.app.store(ref.computerId) !== store) return
      if (action.kind === 'retryRun') await client.call('retryRoutineRun', { botId: ref.botId, runId: action.id })
      else {
        const result = await client.call<{ cancelled: boolean }>('cancelQueuedMessage', { botId: ref.botId, entryId: action.id })
        if (!result.cancelled) throw new Error('This message has already started or left the queue.')
      }
    } catch (error) { if (this.alive && this.app.store(ref.computerId) === store && this.value?.ref === action.ref && this.owner === store) this.fail(message(error)) }
    finally {
      this.mutating = false
      if (this.alive) { if (this.value) this.value = { ...this.value, busy: false }; this.changed(); if (this.snapshot()?.ref === action.ref) await this.load(action.ref) }
    }
  }

  private fail(error: string) { if (this.value) this.value = { ...this.value, actionError: error }; this.changed() }
}
