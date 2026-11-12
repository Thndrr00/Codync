import type { IslandAction } from '../../shared/island-activity'
import { projectIslandGitHub, safeIslandGitHubUrl, type IslandGitHub, type IslandIntegrations, type IslandIntegrationAction } from '../../shared/island-integrations.ts'
import type { Belief } from '../../shared/beliefs'
import type { LibraryItem } from '../../shared/artifacts'
import type { MessageBookmark } from '../../shared/bdots-models'
import type { Entry } from '../../shared/models'
import { parseRef } from '../../shared/bot-ref.ts'
import { Observable } from '../lib/observable.ts'
import type { AppModel } from './app-model'
import type { FlowCanvas } from '../lib/routine-flow'
import type { InstalledConnector } from '../views/marketplace/market-models'
import { islandThreadNavigation } from './island-thread-navigation.ts'
import { revealOwnedSetupTerminal } from '../views/marketplace/setup-terminal-owners.ts'

const message = (error: unknown) => error instanceof Error ? error.message : String(error)
const empty = (ref: string): IslandIntegrations => ({ ref, loading: false, busy: false, fetchedAt: null, errors: [], actionError: null, savedNote: null, github: null, terminals: [], memories: [], connectors: [], artifacts: [], delegations: [], upcoming: [], bookmarks: [], searches: [], search: null })
const actions = new Set(['loadIntegrations', 'reconnectComputer', 'openMemory', 'openConnector', 'openAgentTerminal', 'openIntegrationArtifact', 'openGitHubLink', 'appendScratchpad', 'runSavedSearch', 'openBookmark', 'openSearchHit'])

/** Selected-host reads and explicit intents only; no provider credentials reach the overlay. */
export class IslandIntegrationsStore extends Observable {
  private value: IslandIntegrations | undefined
  private owner: object | undefined
  private generation = 0
  private searchSequence = 0
  private actionSequence = 0
  private alive = true
  private mutation = false
  private revisions = new Map<string, { signature: string; revised: boolean }>()
  private noteAcknowledgements = new Map<string, { owner: object; id: string }>()
  private readonly app: AppModel
  private readonly reveal: (ref: string, connectorId?: string) => void
  private readonly external: (url: string) => void
  constructor(app: AppModel, reveal = (ref: string, connectorId?: string) => window.codync.island.reveal(ref, connectorId), external = (url: string) => window.codync.app.openExternal(url)) {
    super(); this.app = app; this.reveal = reveal; this.external = external
  }

  snapshot() {
    const ref = this.value && parseRef(this.value.ref), store = ref && this.app.store(ref.computerId)
    if (!store || store !== this.owner || !this.value) return undefined
    // Artifact SSE events already update this cache; notices follow revisions without another network read.
    this.value = { ...this.value, artifacts: this.value.artifacts.map((item) => {
      const latest = store.artifacts?.byId.get(item.id)
      if (!latest || latest.botId !== ref!.botId || latest.updatedAt < item.updatedAt) return item
      const signature = `${latest.updatedAt}:${latest.sha256}`, previous = this.revisions.get(item.id)
      const revised = item.revised || (!!previous && previous.signature !== signature)
      this.revisions.set(item.id, { signature, revised })
      return { id: item.id, title: latest.title.slice(0, 180), updatedAt: latest.updatedAt, revised }
    }) }
    return this.value
  }
  dispose() { this.alive = false; ++this.generation; ++this.searchSequence; ++this.actionSequence }
  refresh() { const ref = this.snapshot()?.ref; if (ref && !this.value?.loading) void this.load(ref) }
  reportPollingError(error: unknown) { if (this.alive && this.value) this.fail(`Could not refresh integrations: ${message(error)}`) }

  private adopt(key: string, store: object) {
    if (this.owner === store && this.value?.ref === key) return
    ++this.generation; ++this.searchSequence; ++this.actionSequence
    this.revisions.clear(); this.owner = store
    const acknowledgement = this.noteAcknowledgements.get(key)
    this.value = { ...empty(key), busy: this.mutation, savedNote: acknowledgement?.owner === store ? acknowledgement.id : null }
  }

  async load(key: string) {
    const ref = parseRef(key), store = ref && this.app.store(ref.computerId)
    const bot = ref && store?.bots.get(ref.botId)
    if (!this.alive || !ref || !store || !bot) return
    if (this.snapshot()?.ref === key && this.value?.loading) return
    this.adopt(key, store)
    const generation = ++this.generation
    const current = () => this.alive && generation === this.generation && this.app.store(ref.computerId) === store
    this.value = { ...this.value!, loading: true, errors: [] }; this.changed()
    try {
      if (store.connection.kind !== 'online') throw new Error('Connect this computer to load integrations.')
      const client = await store.ready()
      if (!current()) return
      const results = await Promise.allSettled([
        client.call<IslandGitHub>('islandGitHub', { botId: ref.botId }),
        client.call<{ sessions: IslandIntegrations['terminals'] }>('listAgentTerminals', { backend: bot.backend }),
        client.call<{ beliefs: Belief[] }>('beliefs', { botId: ref.botId, confidence: 'low', forgotten: false }),
        client.call<{ items: InstalledConnector[] }>('connectors'),
        client.library(ref.botId),
        client.call<{ requests: IslandIntegrations['delegations'] }>('islandDelegations', { botId: ref.botId }),
        client.call<FlowCanvas>('routineFlowCanvas', { botId: ref.botId }),
        client.call<{ bookmarks: MessageBookmark[] }>('listBookmarks', { botId: ref.botId }),
        client.call<{ searches: IslandIntegrations['searches'] }>('savedSearches'),
      ] as const)
      if (!current()) return
      const errors: IslandIntegrations['errors'] = []
      const read = <T,>(index: number, section: string, fallback: T): T => {
        const result = results[index]!
        if (result.status === 'fulfilled') return result.value as T
        errors.push({ section, message: message(result.reason) }); return fallback
      }
      const github = read<IslandGitHub | null>(0, 'github', null)
      const terminal = read(1, 'terminals', { sessions: [] as IslandIntegrations['terminals'] })
      const memory = read(2, 'memories', { beliefs: [] as Belief[] })
      const connector = read(3, 'connectors', { items: [] as InstalledConnector[] })
      const library = read(4, 'artifacts', { items: [] as LibraryItem[] })
      // Keep the established viewer's revision cache in sync before opening a published revision.
      for (const item of library.items) if (item.kind === 'artifact') store.artifacts?.upsert(item.artifact)
      const delegation = read(5, 'delegations', { requests: [] as IslandIntegrations['delegations'] })
      const flow = read(6, 'upcoming', { routines: [] as FlowCanvas['routines'] })
      const bookmarks = read(7, 'bookmarks', { bookmarks: [] as MessageBookmark[] })
      const searches = read(8, 'searches', { searches: [] as IslandIntegrations['searches'] })
      if (github?.error) errors.push({ section: 'github', message: github.error })
      const artifacts = library.items.flatMap((item) => {
        if (item.kind !== 'artifact' || item.artifact.botId !== ref.botId) return []
        const artifact = item.artifact, signature = `${artifact.updatedAt}:${artifact.sha256}`
        const previous = this.revisions.get(artifact.id)
        const revised = previous?.revised === true || (!!previous && previous.signature !== signature)
        this.revisions.set(artifact.id, { signature, revised })
        return [{ id: artifact.id, title: artifact.title.slice(0, 180), updatedAt: artifact.updatedAt, revised }]
      }).slice(0, 10)
      const retained = new Set(artifacts.map((item) => item.id))
      if (results[4].status === 'fulfilled') for (const id of this.revisions.keys()) if (!retained.has(id)) this.revisions.delete(id)
      this.value = {
        ...this.value!, loading: false, fetchedAt: Date.now(), errors, github: github ? projectIslandGitHub(github) : null,
        terminals: terminal.sessions.filter((session) => session.backend === bot.backend || session.step === 'node').slice(0, 10).map((session) => ({ id: session.id, backend: session.backend, step: session.step, exitCode: session.exitCode })),
        memories: memory.beliefs.filter((belief) => belief.confidence === 'low').slice(0, 10).map((belief) => ({ file: belief.file, shared: belief.shared, description: belief.description?.slice(0, 180) ?? null, confirmed: belief.confirmed })),
        connectors: connector.items.filter((item) => item.auth === 'signedOut' || item.lastFailure).slice(0, 30).map((item) => ({ id: item.id, name: item.name, auth: item.auth ?? 'none', failure: item.lastFailure?.kind ?? null })),
        artifacts, delegations: delegation.requests.slice(0, 30).map((item) => ({ id: item.id, from: item.from, to: item.to, at: item.at })),
        upcoming: flow.routines.flatMap((card) => card.upcomingRuns.filter((at) => Number.isFinite(at) && at > Date.now()).map((at) => ({ id: card.routine.id, name: card.routine.name, at }))).sort((a, b) => a.at - b.at).slice(0, 10),
        bookmarks: bookmarks.bookmarks.filter((item) => item.botId === ref.botId).slice(0, 10).map((item) => ({ id: item.id, entryId: item.entryId, threadId: item.threadId ?? null, title: (item.note || item.preview || item.author).slice(0, 180) })),
        searches: searches.searches.slice(0, 20).map((item) => ({ id: item.id, query: item.query, scope: item.scope })),
      }
    } catch (error) {
      if (current()) {
        const acknowledgement = this.noteAcknowledgements.get(key)
        this.value = { ...empty(key), busy: this.mutation, savedNote: acknowledgement?.owner === store ? acknowledgement.id : null, errors: [{ section: 'connection', message: message(error) }] }
      }
    }
    finally { if (current()) { this.value = { ...this.value!, loading: false }; this.changed() } }
  }

  async perform(input: IslandAction) {
    if (!actions.has(input.kind) || !('ref' in input)) return
    const action = input as Exclude<IslandIntegrationAction, { kind: 'stopDictation' }>
    if (action.kind === 'loadIntegrations') return this.load(action.ref)
    const ref = parseRef(action.ref), store = ref && this.app.store(ref.computerId)
    if (!this.alive || !ref || !store || !store.bots.has(ref.botId)) return
    if (action.kind === 'reconnectComputer') return this.reconnect(action.ref)
    if (action.kind === 'openMemory') { islandThreadNavigation.open({ kind: 'memory', ref: action.ref }); this.app.select(ref); this.reveal(action.ref); return }
    if (this.snapshot()?.ref !== action.ref || this.owner !== store) return
    if (action.kind === 'openGitHubLink') {
      const gh = this.value!.github
      const links = gh ? [...gh.issues.map((item) => item.url), ...gh.pulls.flatMap((item) => [item.url, ...item.checks.map((check) => check.url)])] : []
      if (safeIslandGitHubUrl(action.url) && links.includes(action.url)) this.external(action.url)
      return
    }
    if (action.kind === 'openConnector') { if (this.value!.connectors.some((item) => item.id === action.id)) this.reveal(action.ref, action.id); return }
    if (action.kind === 'openAgentTerminal') {
      const terminal = this.value!.terminals.find((item) => item.id === action.id)
      if (!terminal) return
      // Closing the original setup screen would terminate its process. Reveal that owner in place.
      try { if (revealOwnedSetupTerminal(store, terminal.id)) return }
      catch (error) { this.fail(message(error)); return }
      const backend = store.hello?.backends.find((item) => item.id === terminal.backend)
      if (!backend && terminal.step !== 'node') { this.fail('This terminal agent is no longer available.'); return }
      islandThreadNavigation.open({ kind: 'terminal', ref: action.ref, id: terminal.id, backend: backend ?? null, step: terminal.step })
      this.app.select(ref); this.reveal(action.ref); return
    }
    if (action.kind === 'openIntegrationArtifact') {
      if (!this.value!.artifacts.some((item) => item.id === action.id)) return
      const revision = this.revisions.get(action.id); if (revision) revision.revised = false
      this.value = { ...this.value!, artifacts: this.value!.artifacts.map((item) => item.id === action.id ? { ...item, revised: false } : item) }
      islandThreadNavigation.open({ kind: 'artifact', ref: action.ref, artifactId: action.id }); this.app.select(ref); this.reveal(action.ref); this.changed(); return
    }
    if (action.kind === 'runSavedSearch') return this.search(action.ref, action.id)
    if (action.kind === 'openBookmark' || action.kind === 'openSearchHit') {
      const hit = action.kind === 'openBookmark' ? this.value!.bookmarks.find((item) => item.id === action.id) : this.value!.search?.hits.find((item) => item.id === action.id)
      if (!hit) return
      const id = 'entryId' in hit ? hit.entryId : hit.id
      const sequence = ++this.actionSequence
      try {
        const client = await store.ready()
        if (!this.current(action.ref, store, sequence)) return
        const result = await client.call<{ entries: Entry[] }>('messageContext', { botId: ref.botId, entryId: id })
        if (!this.current(action.ref, store, sequence)) return
        store.openMessageContext(result.entries, { botId: ref.botId, id, threadId: hit.threadId })
        this.app.select(ref); this.reveal(action.ref)
      } catch (error) { if (this.current(action.ref, store, sequence)) this.fail(message(error)) }
      return
    }
    if (action.kind === 'appendScratchpad') {
      if (this.mutation) return
      this.mutation = true; this.value = { ...this.value!, busy: true, savedNote: null, actionError: null }; this.changed()
      const currentNote = () => this.alive && this.value?.ref === action.ref && this.owner === store && this.app.store(ref.computerId) === store
      try {
        const client = await store.ready()
        if (!currentNote()) return
        await client.call('appendBotScratchpad', { botId: ref.botId, text: action.text })
        if (this.alive && this.app.store(ref.computerId) === store) {
          if (this.noteAcknowledgements.size >= 100) this.noteAcknowledgements.delete(this.noteAcknowledgements.keys().next().value!)
          this.noteAcknowledgements.set(action.ref, { owner: store, id: action.requestId })
        }
        if (currentNote()) this.value = { ...this.value!, savedNote: action.requestId }
      } catch (error) { if (currentNote()) this.fail(message(error)) }
      finally { this.mutation = false; if (this.alive) { if (this.value) this.value = { ...this.value, busy: false }; this.changed() } }
    }
  }

  private current(key: string, store: object, sequence: number) { return this.alive && this.value?.ref === key && this.owner === store && this.app.store(parseRef(key)!.computerId) === store && sequence === this.actionSequence }
  private fail(error: string) { if (this.value) this.value = { ...this.value, actionError: error }; this.changed() }

  private async reconnect(key: string) {
    const ref = parseRef(key)!, store = this.app.store(ref.computerId)!
    if (this.mutation) return
    this.adopt(key, store); this.mutation = true; this.value = { ...this.value!, busy: true, actionError: null }; this.changed()
    const sequence = ++this.actionSequence
    try { await store.refreshStream(); if (this.current(key, store, sequence)) await this.load(key) }
    catch (error) { if (this.current(key, store, sequence)) this.fail(message(error)) }
    finally { this.mutation = false; if (this.alive) { if (this.value) this.value = { ...this.value, busy: false }; this.changed() } }
  }

  private async search(key: string, id: string) {
    const ref = parseRef(key)!, store = this.app.store(ref.computerId)!
    const search = this.value!.searches.find((item) => item.id === id)
    if (!search) return
    const sequence = ++this.searchSequence
    const current = () => this.alive && this.snapshot()?.ref === key && this.owner === store && sequence === this.searchSequence
    this.value = { ...this.value!, search: { id, loading: true, error: null, hits: [] } }; this.changed()
    try {
      const client = await store.ready()
      if (!current()) return
      const page = await client.call<{ results: NonNullable<IslandIntegrations['search']>['hits'] }>('searchMessages', { query: search.query, scope: search.scope, botIds: [ref.botId] })
      if (current()) this.value = { ...this.value!, search: { id, loading: false, error: null, hits: page.results.filter((hit) => hit.botId === ref.botId).slice(0, 10).map((hit) => ({ id: hit.id, botId: hit.botId, threadId: hit.threadId, text: hit.text.slice(0, 180) })) } }
    } catch (error) { if (current()) this.value = { ...this.value!, search: { id, loading: false, error: message(error), hits: [] } } }
    finally { if (current()) this.changed() }
  }
}
