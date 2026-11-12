import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { Entry } from '../../../shared/models.ts'
import type { Artifact } from '../../../shared/artifacts.ts'
import type { AppModel } from '../../store/app-model.ts'
import { IslandIntegrationsStore } from '../../store/island-integrations-store.ts'
import { islandThreadNavigation } from '../../store/island-thread-navigation.ts'
import { ownSetupTerminal } from '../marketplace/setup-terminal-owners.ts'

const ref = 'computer/bot'
const other = 'computer/other'

test('opening a terminal with a live setup owner preserves its navigation and process ownership', async () => {
  const f = fixture()
  await f.model.load(ref)
  let focused = 0
  const release = ownSetupTerminal(f.store, 'login', () => focused++)
  await f.model.perform({ kind: 'openAgentTerminal', ref, id: 'login' })
  assert.equal(focused, 1)
  assert.deepEqual(f.revealed, [])
  assert.deepEqual(f.selected, [])
  assert.equal(islandThreadNavigation.pending, null)
  release(); f.model.dispose()
})
type Request = (method: string, body: Record<string, unknown>) => Promise<unknown>
const artifact = { id: 'artifact', botId: 'bot', title: 'Report', updatedAt: 10, createdAt: 1, size: 10, sha256: 'one', html: 'PRIVATE_ARTIFACT' }
const github = {
  configured: true, repository: 'owner/repo', branch: 'feature', fetchedAt: 10, error: null,
  pulls: [{ number: 1, title: 'PR', state: 'open', draft: false, updatedAt: '2026-01-01', url: 'https://github.com/owner/repo/pull/1', reviewers: ['reviewer'], reviews: [], checks: [{ name: 'CI', status: 'completed', conclusion: 'success', url: 'https://github.com/owner/repo/actions/runs/1' }] }],
  issues: [{ number: 2, title: 'Issue', state: 'open', assignees: ['owner'], updatedAt: '2026-01-01', url: 'https://github.com/owner/repo/issues/2' }],
}
const sourceEntry = { id: 'entry', botId: 'bot', kind: 'agent', seq: 1, rev: 1, turn: 1, data: { text: 'source' }, createdAt: 1, updatedAt: 1 } as Entry

async function response(method: string): Promise<unknown> {
  switch (method) {
    case 'islandGitHub': return structuredClone(github)
    case 'listAgentTerminals': return { sessions: [{ id: 'login', backend: 'claude', step: 'login', exitCode: 0 }, { id: 'different', backend: 'codex', step: 'install', exitCode: null }, { id: 'node', backend: 'node', step: 'node', exitCode: null }] }
    case 'beliefs': return { beliefs: [{ file: 'memory.md', shared: false, description: 'Review this', confidence: 'low', confirmed: null, versions: [{ content: 'PRIVATE_MEMORY' }] }, { file: 'sure.md', shared: true, description: 'Known', confidence: 'high', confirmed: null }] }
    case 'connectors': return { items: [{ id: 'connector', name: 'GitHub', auth: 'signedOut', keys: ['API_KEY'], command: 'PRIVATE_COMMAND', url: 'https://PRIVATE_CONNECTOR', lastFailure: { kind: 'auth', message: 'PRIVATE_FAILURE' } }] }
    case 'library': return { items: [{ kind: 'artifact', at: artifact.updatedAt, artifact: { ...artifact } }] }
    case 'islandDelegations': return { requests: [{ id: 'request', from: 'bot', to: 'peer', at: 1 }] }
    case 'routineFlowCanvas': return { routines: [{ routine: { id: 'routine', name: 'Daily' }, upcomingRuns: [Date.now() - 1000, Date.now() + 60_000, NaN] }] }
    case 'listBookmarks': return { bookmarks: [{ id: 'bookmark', botId: 'bot', entryId: 'entry', threadId: 'thread', note: 'Remember', preview: 'Preview', author: 'Bot' }, { id: 'foreign', botId: 'other', entryId: 'foreign-entry', note: 'Foreign', preview: '', author: 'Other' }] }
    case 'savedSearches': return { searches: [{ id: 'search', query: 'saved query', scope: 'threads' }, { id: 'second', query: 'another query', scope: 'main' }] }
    case 'searchMessages': return { results: [{ id: 'hit', botId: 'bot', threadId: 'thread', text: 'Result'.repeat(100) }, { id: 'foreign-hit', botId: 'other', threadId: null, text: 'PRIVATE_OTHER_BOT' }] }
    case 'messageContext': return { entries: [sourceEntry] }
    case 'appendBotScratchpad': return { saved: true }
    default: throw new Error(`Unexpected host method: ${method}`)
  }
}

function fixture(request: Request = async (method) => response(method)) {
  const pending = islandThreadNavigation.pending
  if (pending) islandThreadNavigation.consume(pending.ref)
  const calls: { method: string; body: Record<string, unknown> }[] = []
  const selected: { computerId: string; botId: string }[] = []
  const revealed: { ref: string; connectorId?: string }[] = []
  const external: string[] = []
  const contexts: { entries: Entry[]; target: { botId: string; id: string; threadId: string | null } }[] = []
  const refreshed: string[] = []
  const invoke = (method: string, body: Record<string, unknown> = {}) => { calls.push({ method, body }); return request(method, body) }
  const client = { library: (botId: string) => invoke('library', { botId }), call: invoke }
  const backend = { id: 'claude', name: 'Claude', available: true, command: 'claude', installHint: '' }
  const store = {
    artifacts: { byId: new Map<string, Artifact>(), upsert(value: Artifact) { this.byId.set(value.id, value) } },
    bots: new Map([['bot', { backend: 'claude' }], ['other', { backend: 'codex' }]]),
    connection: { kind: 'online' }, hello: { backends: [backend] },
    ready: async () => client,
    refreshStream: async () => { refreshed.push('computer') },
    openMessageContext: (entries: Entry[], target: { botId: string; id: string; threadId: string | null }) => contexts.push({ entries, target }),
  }
  const remote = { ...store, refreshStream: async () => { refreshed.push('remote') } }
  const owners = new Map([['computer', store], ['remote', remote]])
  const app = { store: (id: string) => owners.get(id), select: (value: { computerId: string; botId: string }) => selected.push(value) }
  const model = new IslandIntegrationsStore(app as unknown as AppModel, (ref, connectorId) => revealed.push({ ref, connectorId }), (url) => external.push(url))
  return { model, store, owners, client, backend, calls, selected, revealed, external, contexts, refreshed }
}

test('opening a published revision updates the existing viewer cache first', async () => {
  let revision = 10
  const f = fixture(async (method) => method === 'library' ? { items: [{ kind: 'artifact', at: revision, artifact: { ...artifact, updatedAt: revision, sha256: String(revision) } }] } : response(method))
  await f.model.load(ref)
  revision = 20
  await f.model.load(ref)
  assert.equal(f.store.artifacts.byId.get('artifact')?.sha256, '20')
  await f.model.perform({ kind: 'openIntegrationArtifact', ref, id: 'artifact' })
  assert.equal(f.store.artifacts.byId.get('artifact')?.updatedAt, 20)
  f.model.dispose()
})

test('an existing artifact SSE revision appears without another host read', async () => {
  const f = fixture()
  await f.model.load(ref)
  const calls = f.calls.length
  f.store.artifacts.upsert({ ...artifact, updatedAt: 30, sha256: 'sse' })
  const value = f.model.snapshot()!
  assert.equal(value.artifacts[0].revised, true)
  assert.equal(value.artifacts[0].updatedAt, 30)
  assert.equal(f.calls.length, calls)
  f.model.dispose()
})

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (error: Error) => void
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no })
  return { promise, resolve, reject }
}

test('integration reads use existing host APIs with exact bot/backend scopes and bounded presentation metadata', async () => {
  const f = fixture()
  await f.model.load(ref)
  const value = f.model.snapshot()!
  assert.equal(value.loading, false)
  assert.deepEqual(value.errors, [])
  assert.deepEqual(new Set(f.calls.map((call) => call.method)), new Set(['islandGitHub', 'listAgentTerminals', 'beliefs', 'connectors', 'library', 'islandDelegations', 'routineFlowCanvas', 'listBookmarks', 'savedSearches']))
  assert.deepEqual(f.calls.find((call) => call.method === 'listAgentTerminals')?.body, { backend: 'claude' })
  assert.deepEqual(f.calls.find((call) => call.method === 'beliefs')?.body, { botId: 'bot', confidence: 'low', forgotten: false })
  assert.deepEqual(value.terminals.map((item) => item.id), ['login', 'node'])
  assert.deepEqual(value.memories.map((item) => item.file), ['memory.md'])
  assert.deepEqual(value.connectors, [{ id: 'connector', name: 'GitHub', auth: 'signedOut', failure: 'auth' }])
  assert.deepEqual(value.artifacts, [{ id: 'artifact', title: 'Report', updatedAt: 10, revised: false }])
  assert.equal(value.delegations.length, 1)
  assert.equal(value.upcoming.length, 1)
  assert.deepEqual(value.bookmarks.map((item) => item.id), ['bookmark'])
  assert.equal(value.searches.length, 2)
  assert.equal(JSON.stringify(value).includes('PRIVATE_'), false)
  f.model.dispose()
})

test('unsupported sections show scoped errors without replacing successful integrations with fake failures', async () => {
  const f = fixture(async (method) => {
    if (method === 'islandDelegations' || method === 'beliefs') throw new Error(`Unknown method: ${method}`)
    if (method === 'islandGitHub') return { ...structuredClone(github), error: 'GitHub permission denied' }
    return response(method)
  })
  await f.model.load(ref)
  const value = f.model.snapshot()!
  assert.deepEqual(new Set(value.errors.map((error) => error.section)), new Set(['delegations', 'memories', 'github']))
  assert.match(value.errors.find((error) => error.section === 'delegations')!.message, /Unknown method/)
  assert.equal(value.memories.length, 0)
  assert.equal(value.artifacts.length, 1)
  assert.equal(value.github?.pulls.length, 1)
  f.model.dispose()
})

test('artifact revision notices establish a baseline, detect actual revisions and acknowledge only valid opens', async () => {
  let updatedAt = 10
  let sha256 = 'first'
  const f = fixture(async (method) => method === 'library' ? { items: [{ kind: 'artifact', at: updatedAt, artifact: { ...artifact, updatedAt, sha256 } }] } : response(method))
  await f.model.load(ref)
  assert.equal(f.model.snapshot()?.artifacts[0].revised, false)
  await f.model.load(ref)
  assert.equal(f.model.snapshot()?.artifacts[0].revised, false)
  updatedAt = 20; sha256 = 'second'
  await f.model.load(ref)
  assert.equal(f.model.snapshot()?.artifacts[0].revised, true)
  await f.model.perform({ kind: 'openIntegrationArtifact', ref, id: 'unknown' })
  assert.equal(f.model.snapshot()?.artifacts[0].revised, true)
  await f.model.perform({ kind: 'openIntegrationArtifact', ref, id: 'artifact' })
  assert.equal(f.model.snapshot()?.artifacts[0].revised, false)
  assert.equal(islandThreadNavigation.consume(ref)?.kind, 'artifact')
  assert.equal(f.revealed.length, 1)
  await f.model.load(ref)
  assert.equal(f.model.snapshot()?.artifacts[0].revised, false)
  f.model.dispose()
})

test('a transient library failure cannot acknowledge an unseen artifact revision', async () => {
  let updatedAt = 10
  let unavailable = false
  const f = fixture(async (method) => {
    if (method !== 'library') return response(method)
    if (unavailable) throw new Error('Library temporarily unavailable')
    return { items: [{ kind: 'artifact', at: updatedAt, artifact: { ...artifact, updatedAt } }] }
  })
  await f.model.load(ref)
  updatedAt = 20
  await f.model.load(ref)
  assert.equal(f.model.snapshot()?.artifacts[0].revised, true)
  unavailable = true
  await f.model.load(ref)
  assert.equal(f.model.snapshot()?.errors.some((error) => error.section === 'artifacts'), true)
  unavailable = false
  await f.model.load(ref)
  assert.equal(f.model.snapshot()?.artifacts[0].revised, true)
  f.model.dispose()
})

test('only exact reported GitHub links can open and unsafe protocols or unrelated repositories are rejected', async () => {
  const f = fixture()
  await f.model.load(ref)
  for (const url of ['https://github.com/other/repository', 'https://github.com.evil.test/owner/repo/pull/1', 'file:///secret', 'https://user@github.com/owner/repo/pull/1']) await f.model.perform({ kind: 'openGitHubLink', ref, url })
  assert.deepEqual(f.external, [])
  await f.model.perform({ kind: 'openGitHubLink', ref, url: github.pulls[0].checks[0].url })
  assert.deepEqual(f.external, [github.pulls[0].checks[0].url])
  f.model.dispose()
})

test('later mutations of host response objects cannot expand the reported GitHub-link whitelist or leak extra fields', async () => {
  const payload = { ...structuredClone(github), token: 'PRIVATE_TOKEN' }
  const f = fixture(async (method) => {
    if (method === 'islandGitHub') return payload
    if (method === 'listAgentTerminals') return { sessions: [{ id: 'login', backend: 'claude', step: 'login', exitCode: 0, output: 'PRIVATE_TERMINAL' }] }
    if (method === 'islandDelegations') return { requests: [{ id: 'request', from: 'bot', to: 'peer', at: 1, text: 'PRIVATE_DELEGATION' }] }
    return response(method)
  })
  await f.model.load(ref)
  payload.pulls[0].url = 'https://github.com/unreported/repository/pull/1'
  payload.issues.length = 0
  await f.model.perform({ kind: 'openGitHubLink', ref, url: payload.pulls[0].url })
  assert.deepEqual(f.external, [])
  await f.model.perform({ kind: 'openGitHubLink', ref, url: github.issues[0].url })
  assert.deepEqual(f.external, [github.issues[0].url])
  assert.equal(JSON.stringify(f.model.snapshot()).includes('PRIVATE_'), false)
  f.model.dispose()
})

test('late reads cannot overwrite a newer bot or replaced host, and disposal invalidates readiness and fetch completions', async () => {
  const wait = deferred<unknown>()
  const f = fixture(async (method, body) => method === 'islandGitHub' && body.botId === 'bot' ? wait.promise : response(method))
  const loading = f.model.load(ref)
  await Promise.resolve()
  await f.model.load(other)
  wait.resolve(structuredClone(github))
  await loading
  assert.equal(f.model.snapshot()?.ref, other)
  f.model.dispose()

  for (const invalidation of ['replace', 'dispose']) {
    const ready = deferred<ReturnType<typeof fixture>['client']>()
    const f = fixture()
    f.store.ready = () => ready.promise
    const loading = f.model.load(ref)
    if (invalidation === 'replace') f.owners.set('computer', { ...f.store })
    else f.model.dispose()
    ready.resolve(f.client)
    await loading
    assert.equal(f.calls.length, 0, invalidation)
    if (invalidation === 'replace') assert.equal(f.model.snapshot(), undefined)
    f.model.dispose()
  }
})

test('scratchpad capture sends only the selected bot and confirms the matching request after host success', async () => {
  const wait = deferred<unknown>()
  const f = fixture(async (method) => method === 'appendBotScratchpad' ? wait.promise : response(method))
  await f.model.load(ref)
  const saving = f.model.perform({ kind: 'appendScratchpad', ref, text: 'Keep this note', requestId: 'note-1' })
  await Promise.resolve()
  assert.equal(f.model.snapshot()?.busy, true)
  assert.equal(f.model.snapshot()?.savedNote, null)
  assert.deepEqual(f.calls.find((call) => call.method === 'appendBotScratchpad')?.body, { botId: 'bot', text: 'Keep this note' })
  wait.resolve({ saved: true })
  await saving
  assert.equal(f.model.snapshot()?.savedNote, 'note-1')
  assert.equal(f.model.snapshot()?.busy, false)
  f.model.dispose()
})

test('a saved note acknowledgement survives switching bots without acknowledging the other bot', async () => {
  const wait = deferred<unknown>()
  const f = fixture(async (method) => method === 'appendBotScratchpad' ? wait.promise : response(method))
  await f.model.load(ref)
  const saving = f.model.perform({ kind: 'appendScratchpad', ref, text: 'Save once', requestId: 'switching-note' })
  await Promise.resolve()
  await f.model.load(other)
  wait.resolve({ saved: true })
  await saving
  assert.equal(f.model.snapshot()?.savedNote, null)
  f.store.connection.kind = 'offline'
  await f.model.load(ref)
  assert.equal(f.model.snapshot()?.savedNote, 'switching-note', 'an unavailable read must not erase a known successful save')
  f.store.connection.kind = 'online'
  await f.model.load(ref)
  assert.equal(f.model.snapshot()?.savedNote, 'switching-note')
  f.model.dispose()
})

test('disposal and host replacement reject pending fetch results and prevent later explicit actions', async () => {
  for (const invalidation of ['dispose', 'replace']) {
    const wait = deferred<unknown>()
    const f = fixture(async (method) => method === 'library' ? wait.promise : response(method))
    const loading = f.model.load(ref)
    await Promise.resolve()
    if (invalidation === 'dispose') f.model.dispose()
    else f.owners.set('computer', { ...f.store })
    wait.resolve({ items: [{ kind: 'artifact', artifact }] })
    await loading
    assert.equal(f.model.snapshot()?.artifacts.length ?? 0, 0)
    await f.model.perform({ kind: 'openConnector', ref, id: 'connector' })
    assert.equal(f.revealed.length, 0)
    f.model.dispose()
  }
})

test('scratchpad failure remains visible and does not acknowledge unsaved text', async () => {
  const f = fixture(async (method) => { if (method === 'appendBotScratchpad') throw new Error('Disk unavailable'); return response(method) })
  await f.model.load(ref)
  await f.model.perform({ kind: 'appendScratchpad', ref, text: 'Retain this', requestId: 'note-2' })
  assert.equal(f.model.snapshot()?.savedNote, null)
  assert.match(f.model.snapshot()?.actionError ?? '', /Disk unavailable/)
  assert.equal(f.model.snapshot()?.busy, false)
  f.model.dispose()
})

test('opening a bookmark during a pending scratchpad write does not discard the successful note acknowledgement', async () => {
  const wait = deferred<unknown>()
  const f = fixture(async (method) => method === 'appendBotScratchpad' ? wait.promise : response(method))
  await f.model.load(ref)
  const saving = f.model.perform({ kind: 'appendScratchpad', ref, text: 'Save once', requestId: 'concurrent-note' })
  await Promise.resolve()
  await f.model.perform({ kind: 'openBookmark', ref, id: 'bookmark' })
  wait.resolve({ saved: true })
  await saving
  assert.equal(f.model.snapshot()?.savedNote, 'concurrent-note')
  f.model.dispose()
})

test('saved searches use the stored query and scope, constrain results to the selected bot and reject late queries', async () => {
  const wait = deferred<unknown>()
  const f = fixture(async (method, body) => method === 'searchMessages' && body.query === 'saved query' ? wait.promise : response(method))
  await f.model.load(ref)
  await f.model.perform({ kind: 'runSavedSearch', ref, id: 'unknown' })
  assert.equal(f.calls.some((call) => call.method === 'searchMessages'), false)
  const first = f.model.perform({ kind: 'runSavedSearch', ref, id: 'search' })
  await Promise.resolve()
  assert.deepEqual(f.calls.find((call) => call.method === 'searchMessages')?.body, { query: 'saved query', scope: 'threads', botIds: ['bot'] })
  await f.model.perform({ kind: 'runSavedSearch', ref, id: 'second' })
  wait.resolve({ results: [{ id: 'old', botId: 'bot', threadId: null, text: 'Old response' }] })
  await first
  assert.equal(f.model.snapshot()?.search?.id, 'second')
  assert.deepEqual(f.model.snapshot()?.search?.hits.map((hit) => hit.id), ['hit'])
  assert.equal(f.model.snapshot()?.search?.hits[0].text.length, 180)
  f.model.dispose()
})

test('bookmark and saved-search hits open host-validated context for exact retained entries', async () => {
  const f = fixture()
  await f.model.load(ref)
  await f.model.perform({ kind: 'openBookmark', ref, id: 'foreign' })
  assert.equal(f.contexts.length, 0)
  await f.model.perform({ kind: 'openBookmark', ref, id: 'bookmark' })
  assert.deepEqual(f.calls.find((call) => call.method === 'messageContext')?.body, { botId: 'bot', entryId: 'entry' })
  assert.deepEqual(f.contexts[0], { entries: [sourceEntry], target: { botId: 'bot', id: 'entry', threadId: 'thread' } })
  await f.model.perform({ kind: 'runSavedSearch', ref, id: 'search' })
  await f.model.perform({ kind: 'openSearchHit', ref, id: 'foreign-hit' })
  assert.equal(f.contexts.length, 1)
  await f.model.perform({ kind: 'openSearchHit', ref, id: 'hit' })
  assert.equal(f.contexts.length, 2)
  assert.deepEqual(f.contexts[1].target, { botId: 'bot', id: 'hit', threadId: 'thread' })
  assert.equal(f.selected.length, 2)
  assert.equal(f.revealed.length, 2)
  f.model.dispose()
})

test('reconnect targets only the selected computer and then reloads that host', async () => {
  const f = fixture()
  await f.model.perform({ kind: 'reconnectComputer', ref: 'remote/bot' })
  assert.deepEqual(f.refreshed, ['remote'])
  assert.equal(f.model.snapshot()?.ref, 'remote/bot')
  assert.equal(f.model.snapshot()?.busy, false)
  f.model.dispose()
})

test('terminal navigation opens only a reported session with its existing backend and never starts a new session', async () => {
  const f = fixture()
  await f.model.load(ref)
  const before = f.calls.length
  await f.model.perform({ kind: 'openAgentTerminal', ref, id: 'different' })
  assert.equal(f.revealed.length, 0)
  await f.model.perform({ kind: 'openAgentTerminal', ref, id: 'login' })
  const destination = islandThreadNavigation.consume(ref)
  assert.equal(destination?.kind, 'terminal')
  if (destination?.kind === 'terminal') { assert.equal(destination.id, 'login'); assert.equal(destination.backend?.id, 'claude'); assert.equal(destination.step, 'login') }
  assert.equal(f.calls.length, before)
  f.store.hello.backends = []
  await f.model.perform({ kind: 'openAgentTerminal', ref, id: 'login' })
  assert.match(f.model.snapshot()?.actionError ?? '', /no longer available/)
  await f.model.perform({ kind: 'openAgentTerminal', ref, id: 'node' })
  assert.equal(islandThreadNavigation.consume(ref)?.kind, 'terminal')
  f.model.dispose()
})

test('connector reveal accepts only a retained connector on the same selected host', async () => {
  const f = fixture()
  await f.model.load(ref)
  await f.model.perform({ kind: 'openConnector', ref, id: 'unknown' })
  await f.model.perform({ kind: 'openConnector', ref: other, id: 'connector' })
  assert.deepEqual(f.revealed, [])
  await f.model.perform({ kind: 'openConnector', ref, id: 'connector' })
  assert.deepEqual(f.revealed, [{ ref, connectorId: 'connector' }])
  f.owners.set('computer', { ...f.store })
  await f.model.perform({ kind: 'openConnector', ref, id: 'connector' })
  assert.equal(f.revealed.length, 1)
  f.model.dispose()
})
