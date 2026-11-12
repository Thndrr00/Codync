import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { Entry, SharedFile, WeekSummary } from '../../../shared/models.ts'
import type { LibraryItem } from '../../../shared/artifacts.ts'
import type { AppModel } from '../../store/app-model.ts'
import { IslandWorkspaceStore } from '../../store/island-workspace-store.ts'
import { islandThreadNavigation } from '../../store/island-thread-navigation.ts'

type Request = (method: string, body: Record<string, unknown>) => Promise<unknown>
const ref = 'computer/bot'
const file: SharedFile = { id: 'published-file', name: 'report.txt', size: 3, sha256: 'sha' }
const artifact = { id: 'artifact', botId: 'bot', title: 'Report', size: 1, sha256: 'sha', createdAt: 1, updatedAt: 2, html: 'PRIVATE_HTML' }
const diff = { path: 'src/app.ts', added: 1, removed: 0, isNew: false, startLine: 1, patch: 'p'.repeat(4500) }
const entry = (id: string, changes: Partial<Entry> = {}): Entry => ({ id, botId: 'bot', seq: 1, rev: 1, kind: 'agent', turn: 1, data: {}, createdAt: 1, updatedAt: 1, ...changes })
const entries = [
  entry('published-entry', { data: { files: [file] } }),
  entry('proposal', { kind: 'permission', data: { toolKind: 'edit', status: 'pending', diffs: [diff] } }),
  entry('failed', { kind: 'tool', data: { toolKind: 'edit', status: 'failed', diffs: [diff] } }),
  entry('working', { kind: 'tool', data: { toolKind: 'edit', status: 'in_progress', diffs: [diff] } }),
  entry('completed', { kind: 'tool', createdAt: 3, data: { toolKind: 'edit', status: 'completed', diffs: [diff], output: 'PRIVATE_TOOL_OUTPUT' } }),
  entry('read', { kind: 'tool', data: { toolKind: 'read', status: 'completed', diffs: [diff] } }),
]
const library: LibraryItem[] = [
  { kind: 'artifact', at: 2, artifact },
  { kind: 'file', at: 3, entryId: 'published-entry', file },
  { kind: 'attachment', at: 4, entryId: 'attached-entry', file: { id: 'attached', name: 'input.txt', size: 2 } },
]
const day: WeekSummary = { messages: 1, replies: 2, tools: 3, fileEdits: 4, approvalsAsked: 5, approvalsAnswered: 4, approvalsWaiting: 1, activeBots: 2, topBots: [] }

async function response(method: string): Promise<unknown> {
  switch (method) {
    case 'library': return { items: library }
    case 'history': return { entries }
    case 'queuedMessages': return { entries: [{ id: 'queued-z', at: 2 }, { id: 'queued-a', at: 1 }] }
    case 'islandWorkspace': return { git: { branch: 'feature', changes: 2, error: null }, timings: [{ routineId: 'routine', estimateMs: 5000, samples: 5 }] }
    case 'weekSummary': return day
    case 'cancelQueuedMessage': return { cancelled: true }
    case 'retryRoutineRun': return { run: { id: 'new-run' } }
    default: throw new Error(`Unexpected host method: ${method}`)
  }
}

function clearNavigation() {
  const pending = islandThreadNavigation.pending
  if (pending) islandThreadNavigation.consume(pending.ref)
}

function fixture(request: Request = async (method) => response(method)) {
  clearNavigation()
  const calls: { method: string; body: Record<string, unknown> }[] = []
  const selected: { computerId: string; botId: string }[] = []
  const drafts = new Map<string, string>()
  const draftWrites: { botId: string; text: string }[] = []
  const sent: { botId: string; text: string }[] = []
  const invoke = (method: string, body: Record<string, unknown>) => { calls.push({ method, body }); return request(method, body) }
  const client = {
    library: (botId: string) => invoke('library', { botId }),
    call: invoke,
  }
  const store = {
    bots: new Map([['bot', {}], ['other', {}]]), connection: { kind: 'online' },
    ready: async () => client,
    allEntries: () => entries,
    composerDrafts: {
      get: (botId: string) => drafts.get(botId) ?? '',
      set: (botId: string, text: string) => { draftWrites.push({ botId, text }); drafts.set(botId, text) },
    },
    send: (text: string, botId: string) => { sent.push({ text, botId }) },
  }
  const owners = new Map([['computer', store]])
  const app = { store: (id: string) => owners.get(id), select: (value: { computerId: string; botId: string }) => selected.push(value) }
  const opened: string[] = []
  return { model: new IslandWorkspaceStore(app as unknown as AppModel, () => opened.push('opened')), store, owners, client, calls, selected, drafts, draftWrites, sent, opened }
}

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason: Error) => void
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no })
  return { promise, resolve, reject }
}

test('a slow queue mutation does not discard resource navigation or a saved-draft send', async () => {
  const wait = deferred<unknown>()
  const f = fixture(async (method) => method === 'cancelQueuedMessage' ? wait.promise : response(method))
  await f.model.load(ref)
  f.drafts.set('bot', 'original')
  const cancelling = f.model.perform({ kind: 'cancelQueued', ref, id: 'queued-z' })
  await Promise.resolve()
  assert.equal(f.model.snapshot()?.busy, true)
  await f.model.perform({ kind: 'openArtifact', ref, id: 'artifact' })
  assert.equal(islandThreadNavigation.pending?.kind, 'artifact')
  assert.equal(f.selected.length, 1)
  await f.model.perform({ kind: 'sendSavedDraft', ref, text: 'edited', original: 'original' })
  assert.deepEqual(f.sent, [{ botId: 'bot', text: 'edited' }])
  assert.equal(f.drafts.get('bot'), '')
  wait.resolve({ cancelled: true })
  await cancelling
  f.model.dispose()
  clearNavigation()
})

test('workspace reads existing resources, keeps actor queue order and includes completed edits rather than proposals', async () => {
  const f = fixture()
  await f.model.load(ref)
  const snapshot = f.model.snapshot()!
  assert.equal(snapshot.loading, false)
  assert.equal(snapshot.error, null)
  assert.deepEqual(snapshot.artifacts, [{ id: 'artifact', title: 'Report', at: 2 }])
  assert.deepEqual(snapshot.files, [{ id: file.id, entryId: 'published-entry', name: file.name, at: 3 }])
  assert.deepEqual(snapshot.changes.map((item) => item.entryId), ['completed'])
  assert.equal(snapshot.changes[0].diffs[0].patch.length, 4000)
  assert.deepEqual(snapshot.queue.map((item) => item.id), ['queued-z', 'queued-a'])
  assert.equal(snapshot.git?.branch, 'feature')
  assert.equal(snapshot.day?.fileEdits, 4)
  assert.deepEqual(f.calls.find((call) => call.method === 'history')?.body, { botId: 'bot', limit: 100 })
  assert.deepEqual(f.calls.find((call) => call.method === 'weekSummary')?.body, { days: 1 })
  assert.equal(JSON.stringify(snapshot).includes('PRIVATE_HTML'), false)
  assert.equal(JSON.stringify(snapshot).includes('PRIVATE_TOOL_OUTPUT'), false)
  f.model.dispose()
})

test('unsupported host resources show partial errors while successful resources remain usable', async () => {
  const f = fixture(async (method) => {
    if (method === 'islandWorkspace' || method === 'queuedMessages') throw new Error(`Unknown method: ${method}`)
    return response(method)
  })
  await f.model.load(ref)
  const snapshot = f.model.snapshot()!
  assert.match(snapshot.error ?? '', /Unknown method: islandWorkspace/)
  assert.match(snapshot.error ?? '', /Unknown method: queuedMessages/)
  assert.equal(snapshot.artifacts.length, 1)
  assert.equal(snapshot.changes.length, 1)
  assert.equal(snapshot.git, null)
  assert.deepEqual(snapshot.queue, [])
  assert.equal(snapshot.loading, false)
  f.model.dispose()
})

test('delayed resource reads cannot overwrite a newer selected bot', async () => {
  const wait = deferred<unknown>()
  const f = fixture(async (method, body) => method === 'library' && body.botId === 'bot' ? wait.promise : response(method))
  const original = f.model.load(ref)
  await Promise.resolve()
  await f.model.load('computer/other')
  wait.resolve({ items: [] })
  await original
  assert.equal(f.model.snapshot()?.ref, 'computer/other')
  assert.equal(f.model.snapshot()?.artifacts.length, 1)
  f.model.dispose()
})

test('resource reads from a replaced store cannot become the replacement computer snapshot', async () => {
  const wait = deferred<unknown>()
  const f = fixture(async (method) => method === 'library' ? wait.promise : response(method))
  const loading = f.model.load(ref)
  await Promise.resolve()
  f.owners.set('computer', { ...f.store })
  wait.resolve({ items: library })
  await loading
  assert.equal(f.model.snapshot(), undefined)
  f.model.dispose()
})

test('per-entry cancellation uses the acknowledged queue command and reports a message that already started', async () => {
  const f = fixture(async (method) => method === 'cancelQueuedMessage' ? { cancelled: false } : response(method))
  await f.model.perform({ kind: 'cancelQueued', ref, id: 'queued-z' })
  assert.deepEqual(f.calls.find((call) => call.method === 'cancelQueuedMessage')?.body, { botId: 'bot', entryId: 'queued-z' })
  assert.match(f.model.snapshot()?.actionError ?? '', /already started|left the queue/)
  assert.equal(f.calls.some((call) => ['deleteEntry', 'stop', 'stopAll'].includes(call.method)), false)
  assert.equal(f.model.snapshot()?.busy, false)
  f.model.dispose()
})

test('retrying a failed run preserves the original run id through retryRoutineRun', async () => {
  const f = fixture()
  await f.model.perform({ kind: 'retryRun', ref, id: 'failed-run' })
  assert.deepEqual(f.calls.find((call) => call.method === 'retryRoutineRun')?.body, { botId: 'bot', runId: 'failed-run' })
  assert.equal(f.calls.some((call) => call.method === 'runRoutine'), false)
  assert.equal(f.model.snapshot()?.actionError, null)
  f.model.dispose()
})

test('resuming a draft offers its exact text to the island without changing the main app or saved draft', async (t) => {
  const f = fixture()
  const offered: { ref: string; text: string }[] = []
  const previous = Object.getOwnPropertyDescriptor(globalThis, 'window')
  Object.defineProperty(globalThis, 'window', { configurable: true, value: { codync: { island: { offerDraft: (draft: { ref: string; text: string }) => offered.push(draft) } } } })
  t.after(() => { if (previous) Object.defineProperty(globalThis, 'window', previous); else Reflect.deleteProperty(globalThis, 'window') })
  f.drafts.set('bot', 'Saved draft, still editable')
  await f.model.perform({ kind: 'resumeDraft', ref })
  assert.deepEqual(offered, [{ ref, text: 'Saved draft, still editable' }])
  assert.deepEqual(f.selected, [])
  assert.deepEqual(f.calls, [])
  assert.deepEqual(f.sent, [])
  assert.deepEqual(f.draftWrites, [])
  assert.equal(f.drafts.get('bot'), 'Saved draft, still editable')
  await f.model.perform({ kind: 'resumeDraft', ref: 'computer/other' })
  assert.equal(offered.length, 1, 'an empty draft is not offered')
  assert.equal(islandThreadNavigation.pending, null)
  f.model.dispose()
})

test('explicit saved draft send uses the normal send action and clears only an unchanged original draft', async () => {
  const f = fixture()
  f.drafts.set('bot', 'Original draft')
  await f.model.perform({ kind: 'sendSavedDraft', ref, text: 'Edited in the island', original: 'Original draft' })
  assert.deepEqual(f.sent, [{ botId: 'bot', text: 'Edited in the island' }])
  assert.deepEqual(f.draftWrites, [{ botId: 'bot', text: '' }])
  assert.equal(f.drafts.get('bot'), '')
  assert.deepEqual(f.selected, [])
  assert.deepEqual(f.calls, [])
  f.model.dispose()
})

test('sending an island draft preserves newer text typed concurrently in the main composer', async () => {
  const f = fixture()
  f.drafts.set('bot', 'Newer main app draft')
  await f.model.perform({ kind: 'sendSavedDraft', ref, text: 'Explicit island text', original: 'Older draft' })
  assert.deepEqual(f.sent, [{ botId: 'bot', text: 'Explicit island text' }])
  assert.deepEqual(f.draftWrites, [])
  assert.equal(f.drafts.get('bot'), 'Newer main app draft')
  assert.deepEqual(f.selected, [])
  f.model.dispose()
})

test('resource navigation requires the loaded ref and an exact published resource identity', async () => {
  const f = fixture()
  await f.model.load(ref)
  await f.model.perform({ kind: 'openArtifact', ref: 'computer/other', id: 'artifact' })
  await f.model.perform({ kind: 'openArtifact', ref, id: 'unknown' })
  await f.model.perform({ kind: 'openFile', ref, id: file.id, entryId: 'wrong-entry' })
  await f.model.perform({ kind: 'openFile', ref, id: 'attached', entryId: 'attached-entry' })
  assert.deepEqual(f.selected, [])
  assert.equal(islandThreadNavigation.pending, null)
  await f.model.perform({ kind: 'openArtifact', ref, id: 'artifact' })
  assert.equal(islandThreadNavigation.consume(ref)?.kind, 'artifact')
  await f.model.perform({ kind: 'openFile', ref, id: file.id, entryId: 'published-entry' })
  const destination = islandThreadNavigation.consume(ref)
  assert.equal(destination?.kind, 'file')
  if (destination?.kind === 'file') {
    assert.equal(destination.entry.id, 'published-entry')
    assert.equal(destination.file.id, file.id)
  }
  assert.equal(f.selected.length, 2)
  assert.equal(f.opened.length, 2)
  f.owners.set('computer', { ...f.store })
  await f.model.perform({ kind: 'openArtifact', ref, id: 'artifact' })
  assert.equal(f.selected.length, 2)
  f.model.dispose()
})

test('an older published file without a loaded source entry reports a recoverable error', async () => {
  const f = fixture(async (method) => method === 'history' ? { entries: [] } : response(method))
  f.store.allEntries = () => []
  await f.model.load(ref)
  await f.model.perform({ kind: 'openFile', ref, id: file.id, entryId: 'published-entry' })
  assert.match(f.model.snapshot()?.actionError ?? '', /load this older file/)
  assert.deepEqual(f.selected, [])
  assert.equal(islandThreadNavigation.pending, null)
  assert.deepEqual(f.opened, [], 'a failed open keeps the island visible with its error')
  f.model.dispose()
})

test('disposal rejects delayed reads and ignores subsequent draft or mutation intents', async () => {
  const wait = deferred<unknown>()
  const f = fixture(async (method) => method === 'library' ? wait.promise : response(method))
  const loading = f.model.load(ref)
  await Promise.resolve()
  f.model.dispose()
  wait.resolve({ items: library })
  await loading
  assert.deepEqual(f.model.snapshot()?.artifacts, [])
  const count = f.calls.length
  await f.model.perform({ kind: 'resumeDraft', ref })
  await f.model.perform({ kind: 'sendSavedDraft', ref, text: 'Do not send after disposal', original: '' })
  await f.model.perform({ kind: 'cancelQueued', ref, id: 'queued-z' })
  assert.equal(f.calls.length, count)
  assert.deepEqual(f.selected, [])
  assert.deepEqual(f.sent, [])
})

test('a mutation waiting for readiness cannot dispatch after disposal or store replacement', async () => {
  for (const invalidation of ['dispose', 'replace']) {
    const ready = deferred<ReturnType<typeof fixture>['client']>()
    const f = fixture()
    f.store.ready = () => ready.promise
    const running = f.model.perform({ kind: 'cancelQueued', ref, id: 'queued-z' })
    if (invalidation === 'dispose') f.model.dispose()
    else f.owners.set('computer', { ...f.store })
    ready.resolve(f.client)
    await running
    assert.equal(f.calls.some((call) => call.method === 'cancelQueuedMessage'), false, invalidation)
    f.model.dispose()
  }
})

test('a delayed mutation failure cannot appear as an error on a newer bot workspace', async () => {
  const wait = deferred<unknown>()
  const f = fixture(async (method) => method === 'cancelQueuedMessage' ? wait.promise : response(method))
  const mutation = f.model.perform({ kind: 'cancelQueued', ref, id: 'queued-z' })
  await Promise.resolve()
  await f.model.load('computer/other')
  wait.reject(new Error('Previous bot cancellation failed'))
  await mutation
  assert.equal(f.model.snapshot()?.ref, 'computer/other')
  assert.equal(f.model.snapshot()?.actionError, null)
  f.model.dispose()
})
