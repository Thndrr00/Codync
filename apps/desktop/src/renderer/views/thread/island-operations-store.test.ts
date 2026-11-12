import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { AppModel } from '../../store/app-model.ts'
import type { SSHState } from '../../../shared/ssh.ts'
import { IslandOperationsStore } from '../../store/island-operations-store.ts'
import { islandThreadNavigation } from '../../store/island-thread-navigation.ts'

const ref = 'computer/bot'
const other = 'computer/other'
type Request = (method: string, body: Record<string, unknown>) => Promise<unknown>
const miniApp = { slug: 'dashboard', botId: 'bot', title: 'Dashboard', version: 2, entry: 'index.html', createdAt: 1, updatedAt: 2, ownerToken: 'PRIVATE_OWNER', url: 'https://host/app?token=PRIVATE_CAPABILITY', links: [{ token: 'PRIVATE_LINK' }] }
const bot = { id: 'bot', agentStep: { agent: 'codex', model: 'model' as string | null, reason: 'quota' as string | null, since: 10 } }

async function response(method: string): Promise<unknown> {
  switch (method) {
    case 'islandRuntime': return { context: { used: 200, size: 1000, lastTurn: 2, compactions: 3, content: 'PRIVATE_CONTEXT' }, cost: { amount: 99, currency: 'FAKE_PRICE' }, mcp: [{ id: 'call', connectorId: 'connector', tool: 'read', status: 'running', createdAt: 2, arguments: 'PRIVATE_ARGUMENTS', result: 'PRIVATE_RESULT' }], error: null }
    case 'botUsage': return { used: 17, limit: 100, fleetUsed: 80, fleetLimit: 1000, resetsAt: 100, private: 'PRIVATE_USAGE' }
    case 'tokenBudgets': return { budgets: [{ botId: 'bot', used: 17, limit: 100, level: 'exceeded' }, { botId: '*', used: 80, limit: 1000, level: 'warning' }, { botId: 'other', used: 999, limit: 1, level: 'exceeded' }] }
    case 'hostHealth': return { cpu: 0, ramUsedPercent: 30, ramFree: 700, ramTotal: 1000, diskFree: 500, diskTotal: 2000, at: 50, agents: [{ botId: 'other', pid: 999, rss: 800 }, { botId: 'bot', pid: 12345, rss: 200 }], private: 'PRIVATE_HEALTH' }
    case 'islandResources': return { documents: [{ id: 'document', title: 'Recent', updatedAt: 20, body: 'PRIVATE_DOCUMENT' }], channelFailures: [{ id: 'delivery', source: 'slack', state: 'unknown', text: 'PRIVATE_DELIVERY' }], channelState: [{ source: 'discord', state: 'error' }], scope: 'computer', error: null }
    case 'islandGit': return { root: '/project', conflicts: ['src/a.ts'], worktrees: [{ path: '/worktree', branch: 'feature', head: 'abc', detached: false, locked: false, prunable: false, botIds: ['other', 'deleted'], command: 'PRIVATE_COMMAND' }], error: null }
    case 'listBotApps': return { apps: [{ ...miniApp }, { ...miniApp, slug: 'foreign', botId: 'other' }] }
    case 'computerPanel': return { enabled: true, permissions: { browser: true, files: true, shell: true }, workspace: '/project', screen: { userControl: true, agentBot: 'bot', computerUse: true, protectedEntry: { id: 'PRIVATE_ENTRY', site: 'PRIVATE_SITE' } }, activity: [{ output: 'PRIVATE_OUTPUT' }] }
    case 'hostUpdateStatus': return { currentVersion: '2.44.2', state: { phase: 'downloading', availableVersion: '2.45.0', error: null, token: 'PRIVATE_UPDATE' } }
    default: throw new Error(`Unexpected host method: ${method}`)
  }
}

function clearPending() { const pending = islandThreadNavigation.pending; if (pending) islandThreadNavigation.consume(pending.ref) }
function ssh(kind: string): SSHState {
  return { profiles: [{ id: 'ssh', name: 'SSH', host: 'example', port: null, user: null, identityFile: null, remotePort: 19222, computerId: 'computer' }], status: { ssh: { kind } }, attachments: [], needsLinger: [] } as unknown as SSHState
}
function fixture(request: Request = async (method) => response(method)) {
  clearPending()
  const calls: { method: string; body: Record<string, unknown> }[] = []
  const selected: { computerId: string; botId: string }[] = []
  const revealed: string[] = []
  const browser = { focused: 0, checked: 0, exists: true }
  const sshState = { current: ssh('failed'), requested: 0, connected: [] as string[] }
  const client = { call: (method: string, body: Record<string, unknown> = {}) => { calls.push({ method, body }); return request(method, body) } }
  const entries = [{ id: 'compact', createdAt: 12, data: { compaction: { byHost: true }, text: 'PRIVATE_COMPACTION' } }, { id: 'message', createdAt: 13, data: { text: 'PRIVATE_MESSAGE' } }]
  const documents = { busy: false, mutating: false, error: null as string | null, draft: null as { id: string } | null, resume() {}, async open(id: string) { this.draft = { id } } }
  const store = { computer: { id: 'computer' }, bots: new Map([['bot', { ...bot }], ['other', { ...bot, id: 'other' }]]), connection: { kind: 'online' }, isLoopback: true, ready: async () => client, allEntries: () => entries, documentsFor: () => documents }
  const owners = new Map([['computer', store]])
  const app = { local: store as typeof store | undefined, store: (id: string) => owners.get(id), select: (value: { computerId: string; botId: string }) => selected.push(value) }
  const model = new IslandOperationsStore(app as unknown as AppModel, {
    reveal: (key) => revealed.push(key),
    browserState: async () => { browser.checked++; return browser.exists },
    focusBrowser: async () => { browser.focused++; return browser.exists },
    sshState: async () => { sshState.requested++; return sshState.current },
    connectSSH: (id) => { sshState.connected.push(id) },
  })
  return { model, app, store, owners, client, calls, selected, revealed, browser, sshState, documents }
}
function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (error: Error) => void
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no })
  return { promise, resolve, reject }
}

test('a document save in progress leaves navigation unacknowledged with a visible retry reason', async () => {
  const f = fixture()
  await f.model.load(ref)
  f.documents.busy = true
  await f.model.perform({ kind: 'openOperationDocument', ref, id: 'document' })
  assert.equal(islandThreadNavigation.pending, null)
  assert.deepEqual(f.revealed, [])
  assert.match(f.model.snapshot()?.actionError ?? '', /save to finish/)
  f.documents.busy = false
  await f.model.perform({ kind: 'openOperationDocument', ref, id: 'document' })
  assert.equal(f.documents.draft?.id, 'document')
  assert.equal(islandThreadNavigation.consume(ref, f.store)?.kind, 'operationPanel')
  f.model.dispose()
})

test('an unavailable document or replaced owner cannot reveal stale editor content', async () => {
  const f = fixture()
  await f.model.load(ref)
  f.documents.draft = { id: 'document' }
  f.documents.open = async () => { f.documents.error = 'Document was deleted' }
  await f.model.perform({ kind: 'openOperationDocument', ref, id: 'document' })
  assert.match(f.model.snapshot()?.actionError ?? '', /deleted/)
  assert.deepEqual(f.revealed, [])
  const wait = deferred<void>()
  f.documents.open = async (id) => { await wait.promise; f.documents.draft = { id } }
  const opening = f.model.perform({ kind: 'openOperationDocument', ref, id: 'document' })
  f.owners.set('computer', { ...f.store })
  wait.resolve()
  await opening
  assert.deepEqual(f.revealed, [])
  assert.equal(islandThreadNavigation.pending, null)
  f.model.dispose()
})

test('operations read nine existing host reports and publish bounded metadata without private payloads', async () => {
  const f = fixture()
  await f.model.load(ref)
  const value = f.model.snapshot()!
  assert.equal(f.calls.length, 9)
  assert.deepEqual(value.errors, [])
  assert.deepEqual(value.runtime?.context, { used: 200, size: 1000, lastTurn: 2, compactions: 3 })
  assert.deepEqual(value.runtime?.mcp, [{ id: 'call', connectorId: 'connector', tool: 'read', status: 'running', createdAt: 2 }])
  assert.deepEqual(value.apps, [{ slug: 'dashboard', title: 'Dashboard', version: 2, updatedAt: 2 }])
  assert.deepEqual(value.computer, { enabled: true, userControl: true, agentBot: 'bot', computerUse: true })
  assert.equal(value.health?.rss, 200)
  assert.equal(value.health?.cpu, 0)
  assert.equal(JSON.stringify(value).includes('PRIVATE'), false)
  assert.equal(JSON.stringify(value).includes('pid'), false)
  f.model.dispose()
})

test('usage and budget levels come from the host while unsupported monetary costs stay unknown', async () => {
  const f = fixture()
  await f.model.load(ref)
  const value = f.model.snapshot()!
  assert.deepEqual(value.usage, { used: 17, limit: 100, fleetUsed: 80, fleetLimit: 1000, resetsAt: 100 })
  assert.deepEqual(value.budgets.map((budget) => [budget.botId, budget.level]), [['bot', 'exceeded'], ['*', 'warning']])
  assert.deepEqual(value.runtime?.cost, { amount: null, currency: null, reason: 'notReported' })
  assert.equal(JSON.stringify(value).includes('FAKE_PRICE'), false)
  f.model.dispose()
})

test('fallback and compaction signals follow the current bot without exposing transcript content', async () => {
  const f = fixture()
  await f.model.load(ref)
  assert.deepEqual(f.model.snapshot()?.fallback, { agent: 'codex', model: 'model', reason: 'quota', since: 10 })
  assert.deepEqual(f.model.snapshot()?.compactions, [{ id: 'compact', at: 12, byHost: true }])
  f.store.bots.set('bot', { ...bot, agentStep: { agent: 'claude', model: null, reason: null, since: 20 } })
  assert.equal(f.model.snapshot()?.fallback?.agent, 'claude')
  f.model.dispose()
})

test('partial failures retain successful reports and explicit section errors', async () => {
  const f = fixture(async (method) => {
    if (method === 'hostHealth') throw new Error('Unknown hostHealth')
    if (method === 'islandGit') return { root: '/project', conflicts: [], worktrees: [], error: 'Git unavailable' }
    return response(method)
  })
  await f.model.load(ref)
  const value = f.model.snapshot()!
  assert.equal(value.health, null)
  assert.equal(value.apps.length, 1)
  assert.equal(value.usage?.used, 17)
  assert.deepEqual(new Set(value.errors.map((error) => error.section)), new Set(['health', 'git']))
  f.model.dispose()
})

test('offline computers show a connection error without calling host methods', async () => {
  const f = fixture()
  f.store.connection.kind = 'offline'
  await f.model.load(ref)
  assert.deepEqual(f.calls, [])
  assert.match(f.model.snapshot()?.errors[0]?.message ?? '', /Connect this computer/)
  assert.equal(f.model.snapshot()?.loading, false)
  f.model.dispose()
})

test('remote control connections never request the loopback-only host update report', async () => {
  const f = fixture()
  f.store.isLoopback = false
  f.app.local = undefined
  await f.model.load(ref)
  assert.equal(f.calls.length, 8)
  assert.equal(f.calls.some((call) => call.method === 'hostUpdateStatus'), false)
  assert.equal(f.model.snapshot()?.hostUpdate, null)
  assert.equal(f.model.snapshot()?.localComputer, false)
  f.model.dispose()
})

test('old reads cannot overwrite a newer selected bot', async () => {
  const wait = deferred<unknown>()
  const f = fixture(async (method, body) => method === 'islandRuntime' && body.botId === 'bot' ? wait.promise : response(method))
  const old = f.model.load(ref)
  await Promise.resolve()
  await f.model.load(other)
  wait.resolve(await response('islandRuntime'))
  await old
  assert.equal(f.model.snapshot()?.ref, other)
  f.model.dispose()
})

test('store replacement and disposal reject requests waiting for readiness', async () => {
  for (const invalidation of ['replace', 'dispose']) {
    const f = fixture()
    const ready = deferred<typeof f.client>()
    f.store.ready = () => ready.promise
    const loading = f.model.load(ref)
    if (invalidation === 'replace') f.owners.set('computer', { ...f.store })
    else f.model.dispose()
    ready.resolve(f.client)
    await loading
    assert.equal(f.calls.length, 0)
    f.model.dispose()
  }
})

test('store replacement and disposal reject delayed report results and later navigation', async () => {
  for (const invalidation of ['replace', 'dispose']) {
    const f = fixture(async (method) => method === 'islandRuntime' ? wait.promise : response(method))
    const wait = deferred<unknown>()
    const loading = f.model.load(ref)
    await Promise.resolve()
    if (invalidation === 'replace') f.owners.set('computer', { ...f.store })
    else f.model.dispose()
    wait.resolve(await response('islandRuntime'))
    await loading
    await f.model.perform({ kind: 'openOperationApp', ref, id: 'dashboard' })
    assert.deepEqual(f.revealed, [])
    assert.equal(f.model.snapshot()?.runtime ?? null, null)
    f.model.dispose()
  }
})

test('document and mini-app navigation accepts only exact resources from the selected snapshot', async () => {
  const f = fixture()
  await f.model.load(ref)
  await f.model.perform({ kind: 'openOperationDocument', ref, id: 'unknown' })
  await f.model.perform({ kind: 'openOperationApp', ref, id: 'foreign' })
  await f.model.perform({ kind: 'openOperationApp', ref: other, id: 'dashboard' })
  assert.equal(islandThreadNavigation.pending, null)
  await f.model.perform({ kind: 'openOperationDocument', ref, id: 'document' })
  assert.equal(islandThreadNavigation.consume(ref, f.store)?.kind, 'operationPanel')
  await f.model.perform({ kind: 'openOperationApp', ref, id: 'dashboard' })
  const destination = islandThreadNavigation.consume(ref, f.store)
  assert.equal(destination?.kind, 'miniApp')
  if (destination?.kind === 'miniApp') assert.equal(destination.app.ownerToken, 'PRIVATE_OWNER')
  assert.equal(JSON.stringify(f.model.snapshot()).includes('PRIVATE_OWNER'), false)
  assert.equal(f.selected.length, 2)
  f.model.dispose()
})

test('worktree navigation selects only an existing bot on that same computer and changes no folder', async () => {
  const f = fixture()
  await f.model.load(ref)
  const reads = f.calls.length
  await f.model.perform({ kind: 'selectWorktreeBot', ref, id: 'deleted' })
  await f.model.perform({ kind: 'selectWorktreeBot', ref, id: 'computer/other' })
  await f.model.perform({ kind: 'selectWorktreeBot', ref, id: 'unknown' })
  assert.equal(f.selected.length, 0)
  await f.model.perform({ kind: 'selectWorktreeBot', ref, id: 'other' })
  assert.deepEqual(f.selected, [{ computerId: 'computer', botId: 'other' }])
  assert.deepEqual(f.revealed, ['computer/other'])
  assert.equal(f.calls.length, reads)
  f.model.dispose()
})

test('browser focus is local-only and an absent window does not create a session or lease', async () => {
  const f = fixture()
  await f.model.load(ref)
  const reads = f.calls.length
  f.browser.exists = false
  await f.model.perform({ kind: 'openExistingBrowser', ref })
  assert.equal(f.browser.focused, 1)
  assert.match(f.model.snapshot()?.actionError ?? '', /No browser window is open/)
  assert.equal(f.calls.length, reads)
  f.app.local = undefined
  await f.model.perform({ kind: 'openExistingBrowser', ref })
  assert.equal(f.browser.focused, 1)
  assert.match(f.model.snapshot()?.actionError ?? '', /this computer only/)
  f.model.dispose()
})

test('late browser failure cannot replace a newer bot action state', async () => {
  const wait = deferred<boolean>()
  const f = fixture()
  const model = new IslandOperationsStore(f.app as unknown as AppModel, { reveal: () => {}, browserState: async () => true, focusBrowser: () => wait.promise })
  await model.load(ref)
  const focusing = model.perform({ kind: 'openExistingBrowser', ref })
  await model.load(other)
  wait.reject(new Error('Old browser failure'))
  await focusing
  assert.equal(model.snapshot()?.ref, other)
  assert.equal(model.snapshot()?.actionError, null)
  model.dispose(); f.model.dispose()
})

test('SSH retry connects only a known safe retry state and never installs or trusts a key', async () => {
  const f = fixture()
  await f.model.load(ref)
  for (const kind of ['idle', 'failed', 'retrying']) {
    f.sshState.current = ssh(kind)
    await f.model.perform({ kind: 'retryIslandSSH', id: 'ssh' })
  }
  assert.deepEqual(f.sshState.connected, ['ssh', 'ssh', 'ssh'])
  for (const kind of ['confirmHostKey', 'notInstalled', 'installing', 'connecting', 'connected']) {
    f.sshState.current = ssh(kind)
    await f.model.perform({ kind: 'retryIslandSSH', id: 'ssh' })
  }
  await f.model.perform({ kind: 'retryIslandSSH', id: 'unknown' })
  assert.equal(f.sshState.connected.length, 3)
  assert.match(f.model.snapshot()?.actionError ?? '', /Open Computers/)
  f.model.dispose()
})

test('a disposed SSH retry awaiting state cannot dispatch and duplicate clicks share one retry', async () => {
  const f = fixture()
  const wait = deferred<SSHState>()
  let requested = 0
  const connected: string[] = []
  const model = new IslandOperationsStore(f.app as unknown as AppModel, { sshState: () => { requested++; return wait.promise }, connectSSH: (id) => { connected.push(id) } })
  const first = model.perform({ kind: 'retryIslandSSH', id: 'ssh' })
  await model.perform({ kind: 'retryIslandSSH', id: 'ssh' })
  assert.equal(requested, 1)
  model.dispose()
  wait.resolve(ssh('failed'))
  await first
  assert.deepEqual(connected, [])
  f.model.dispose()
})


test('a later panel, mini-app or worktree navigation wins over a slow document read', async () => {
  for (const action of [
    { kind: 'openOperationPanel', ref, panel: 'trace' },
    { kind: 'openOperationPanel', ref, panel: 'computer' },
    { kind: 'openOperationPanel', ref, panel: 'budget' },
    { kind: 'openOperationApp', ref, id: 'dashboard' },
    { kind: 'selectWorktreeBot', ref, id: 'other' },
  ] as const) {
    const f = fixture()
    await f.model.load(ref)
    const wait = deferred<void>()
    f.documents.open = async (id) => { await wait.promise; f.documents.draft = { id } }
    const opening = f.model.perform({ kind: 'openOperationDocument', ref, id: 'document' })
    await f.model.perform(action)
    const destination = islandThreadNavigation.pending
    wait.resolve()
    await opening
    assert.equal(islandThreadNavigation.pending, destination)
    assert.deepEqual(f.revealed, [action.kind === 'selectWorktreeBot' ? other : ref])
    assert.equal(f.selected.length, 1)
    f.model.dispose()
  }
})
