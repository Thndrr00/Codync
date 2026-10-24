import { test } from 'node:test'
import assert from 'node:assert/strict'
import type { HostClient } from '../client/host-client'
import { DocumentsStore, type Document } from '../store/documents-store.ts'

const document: Document = {
  id: 'page', botId: 'bot', parentId: null, title: 'Note', body: 'Original',
  revision: 1, rev: 1, createdAt: 1, updatedAt: 1,
}

function model(call: (method: string, body: Record<string, unknown>) => Promise<unknown>, scope = 'test-context/host', storage = new Map<string, string>()) {
  Object.defineProperty(globalThis, 'localStorage', { configurable: true, value: {
    getItem: (key: string) => storage.get(key) ?? null,
    setItem: (key: string, value: string) => storage.set(key, value),
  } })
  return new DocumentsStore('bot', async () => ({ call }) as unknown as HostClient, scope)
}

function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((done) => { resolve = done })
  return { promise, resolve }
}

test('host title normalization finishes autosave without creating more revisions', async () => {
  let saves = 0
  const store = model(async (method) => {
    if (method === 'saveDocument') { saves++; return { status: 'saved', document } }
    return { items: [], next: null }
  })
  try {
    store.create()
    store.edit({ title: '  Note  ', body: 'Original' })
    const session = store.editorSession
    await store.save()
    assert.equal(store.editorSession, session, 'creation confirmation must preserve editor identity and focus')
    assert.equal(store.dirty, false)
    assert.equal(store.draft?.title, 'Note')
    await store.save()
    assert.equal(saves, 1)
  } finally { store.dispose() }
})

test('edits made during a save retain text and adopt the saved identity and revision', async () => {
  const pending = deferred<unknown>()
  const store = model(async (method) => method === 'saveDocument' ? pending.promise : { items: [], next: null })
  try {
    store.create()
    store.edit({ title: 'Note', body: 'Original' })
    const saving = store.save()
    store.edit({ body: 'Newer text' })
    pending.resolve({ status: 'saved', document })
    await saving
    assert.equal(store.draft?.body, 'Newer text')
    assert.equal(store.draft?.id, 'page')
    assert.equal(store.draft?.ifRevision, 1)
    assert.equal(store.dirty, true)
  } finally { store.dispose() }
})

test('an unconfirmed creation retries the same nonce and preserves a newer draft on replay', async () => {
  const nonces: unknown[] = []
  const store = model(async (method, body) => {
    if (method !== 'saveDocument') return { items: [], next: null }
    nonces.push(body.clientNonce)
    if (nonces.length === 1) throw new Error('connection lost after write')
    return { status: 'saved', document }
  })
  try {
    store.create()
    store.edit({ title: 'Note', body: 'Original' })
    await store.save()
    store.edit({ body: 'After disconnection' })
    await store.save()
    assert.equal(typeof nonces[0], 'string')
    assert.equal(nonces[1], nonces[0])
    assert.equal(store.draft?.id, 'page')
    assert.equal(store.draft?.body, 'After disconnection')
    assert.equal(store.dirty, true)
  } finally { store.dispose() }
})

test('move locks editing until its response and then permits edits again', async () => {
  const pending = deferred<unknown>()
  const store = model(async (method) => {
    if (method === 'getDocument') return { document }
    if (method === 'moveDocument') return pending.promise
    return { items: [], next: null }
  })
  try {
    await store.open('page')
    const moving = store.move('parent')
    assert.equal(store.mutating, true)
    store.edit({ body: 'Must not replace an in-flight move' })
    assert.equal(store.draft?.body, 'Original')
    pending.resolve({ status: 'saved', document: { ...document, parentId: 'parent', revision: 2 } })
    await moving
    assert.equal(store.mutating, false)
    store.edit({ body: 'Allowed after move' })
    assert.equal(store.draft?.body, 'Allowed after move')
    assert.equal(store.draft?.ifRevision, 2)
  } finally { store.dispose() }
})

test('CAS conflict keeps the local draft until the user chooses the latest version', async () => {
  const latest = { ...document, title: 'Changed elsewhere', revision: 2 }
  const store = model(async (method) => {
    if (method === 'getDocument') return { document }
    if (method === 'saveDocument') return { status: 'conflict', document: latest }
    return { items: [], next: null }
  })
  try {
    await store.open('page')
    store.edit({ body: 'Local work' })
    await store.save()
    assert.equal(store.draft?.body, 'Local work')
    assert.equal(store.conflict?.revision, 2)
    store.reloadConflict()
    assert.equal(store.draft?.title, 'Changed elsewhere')
    assert.equal(store.dirty, false)
  } finally { store.dispose() }
})

test('closing the panel while a save is pending prevents later autosave dispatch', async (context) => {
  context.mock.timers.enable({ apis: ['setTimeout'] })
  const pending = deferred<unknown>()
  let saves = 0
  const store = model(async (method) => {
    if (method === 'saveDocument') { saves++; return pending.promise }
    return { items: [], next: null }
  })
  try {
    store.create()
    store.edit({ title: 'Note', body: 'Original' })
    const saving = store.save()
    store.edit({ body: 'Retained when closed' })
    store.dispose()
    pending.resolve({ status: 'saved', document })
    await saving
    context.mock.timers.tick(2_000)
    await Promise.resolve()
    assert.equal(saves, 1)
    assert.equal(store.draft?.body, 'Retained when closed')
    assert.equal(store.dirty, true)
  } finally { store.dispose() }
})

test('durable drafts survive reopening and remain isolated by account and computer', async () => {
  const storage = new Map<string, string>()
  const call = async () => ({ items: [], next: null })
  const first = model(call, 'account-a/host-a', storage)
  first.create(); first.edit({ title: 'Recovered', body: '日本語 **body**' }); first.dispose()
  const reopened = model(call, 'account-a/host-a', storage)
  const otherHost = model(call, 'account-a/host-b', storage)
  const otherAccount = model(call, 'account-b/host-a', storage)
  try {
    assert.equal(otherHost.pendingDrafts.length, 0)
    assert.equal(otherAccount.pendingDrafts.length, 0)
    await reopened.recoverDraft('new')
    assert.equal(reopened.draft?.body, '日本語 **body**')
    assert.equal(reopened.dirty, true)
  } finally { first.retire(); reopened.retire(); otherHost.retire(); otherAccount.retire() }
})

test('a retired store cannot erase newer durable text when its old save completes', async () => {
  const storage = new Map<string, string>()
  const pending = deferred<unknown>()
  const first = model(async (method) => method === 'saveDocument' ? pending.promise : { items: [], next: null }, 'scope', storage)
  first.create(); first.edit({ title: 'Note', body: 'Original' })
  const saving = first.save()
  first.retire()
  const replacement = model(async () => ({ items: [], next: null }), 'scope', storage)
  try {
    await replacement.recoverDraft('new')
    replacement.edit({ body: 'New account-session text' }); replacement.dispose()
    pending.resolve({ status: 'saved', document }); await saving
    const snapshot = [...storage.values()][0]
    assert.ok(snapshot.includes('New account-session text'))
  } finally { replacement.retire() }
})

test('quiet refresh retains explicitly loaded document pages', async () => {
  const pages = Array.from({ length: 200 }, (_, index) => ({ ...document, id: `page-${index}` }))
  const store = model(async (_method, body) => body.afterId
    ? { items: pages.slice(100), next: null } : { items: pages.slice(0, 100), next: 'page-99' })
  try {
    await store.load(); await store.load(true)
    assert.equal(store.items.length, 200)
    await store.load(false, true)
    assert.equal(store.items.length, 200)
    assert.equal(store.next, null)
  } finally { store.dispose() }
})

test('saving a copy creates a new identity without overwriting the original page', async () => {
  const sent: Record<string, unknown>[] = []
  const store = model(async (method, body) => {
    if (method === 'getDocument') return { document }
    if (method === 'saveDocument') { sent.push(body); return { status: 'saved', document: { ...document, id: 'copy' } } }
    return { items: [], next: null }
  })
  try {
    await store.open('page'); store.saveAsCopy()
    // Wait for the queued host intent and its reply.
    await new Promise((resolve) => setImmediate(resolve))
    assert.equal(sent.length, 1)
    assert.equal(sent[0].id, undefined)
    assert.equal(sent[0].ifRevision, 0)
    assert.equal(typeof sent[0].clientNonce, 'string')
    assert.equal(store.draft?.id, 'copy')
  } finally { store.dispose() }
})

test('recovering a draft supersedes an older open without leaving the editor loading', async () => {
  const pending = deferred<unknown>()
  const store = model(async () => pending.promise)
  try {
    store.create(); store.edit({ title: 'Local draft', body: 'Keep me' }); store.dispose(); store.resume()
    const opening = store.open('other-page')
    assert.equal(store.opening, true)
    await store.recoverDraft('new')
    assert.equal(store.opening, false)
    pending.resolve({ document }); await opening
    assert.equal(store.draft?.title, 'Local draft')
  } finally { store.retire() }
})
