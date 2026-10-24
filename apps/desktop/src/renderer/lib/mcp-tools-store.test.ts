import { test } from 'node:test'
import assert from 'node:assert/strict'
import type { HostClient } from '../client/host-client'
import { McpToolsStore, type ToolRequest } from '../store/mcp-tools-store.ts'

const row: ToolRequest = { id: 'call', botId: 'bot', tool: 'write', status: 'pending', createdAt: 1, expiresAt: 1000 }
function fixture(handle?: (method: string, body: Record<string, unknown>) => Promise<unknown>) {
  const calls: { method: string; body: Record<string, unknown> }[] = []
  const client = { call: async (method: string, body: Record<string, unknown>) => {
    calls.push({ method, body })
    if (handle) return handle(method, body)
    if (method === 'getMcpToolSettings') return { items: [{ tool: 'write', mode: 'ask' }] }
    if (method === 'listMcpToolCalls') return { items: [row], next: null }
    if (method === 'getMcpToolCall') return { ...row, arguments: { text: 'original' } }
    return {}
  } } as unknown as HostClient
  return { model: new McpToolsStore('connector', async () => client), calls }
}
function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>((done) => { resolve = done }); return { promise, resolve } }

test('policies and requests load concurrently and remain scoped to the selected connection', async () => {
  const { model, calls } = fixture()
  await model.load()
  assert.equal(model.policies[0].mode, 'ask')
  assert.deepEqual(model.requests, [row])
  assert.ok(calls.every((call) => call.body.connectorId === 'connector'))
})
test('approval sends the immutable request identity and decision only', async () => {
  const { model, calls } = fixture()
  await model.inspect(row)
  assert.deepEqual(model.detail?.arguments, { text: 'original' })
  await model.decide(row.id, true)
  assert.deepEqual(calls.find((call) => call.method === 'resolveToolCall')?.body, { id: 'call', approve: true })
  assert.equal(model.detail, null)
})
test('closing an inspection ignores the response that arrives afterward', async () => {
  const pending = deferred<unknown>()
  const { model } = fixture(async () => pending.promise)
  const opening = model.inspect(row)
  model.closeDetail()
  pending.resolve({ ...row, arguments: { text: 'late' } })
  await opening
  assert.equal(model.detail, null)
  assert.equal(model.inspecting, false)
})
test('disposing prevents old loads from replacing state and resuming permits a fresh load', async () => {
  const pending = deferred<unknown>()
  let delayed = true
  const { model } = fixture(async () => delayed ? pending.promise : { items: [], next: null })
  const loading = model.load()
  model.dispose()
  pending.resolve({ items: [row], next: null })
  await loading
  assert.deepEqual(model.requests, [])
  delayed = false; model.resume()
  await model.load()
  assert.equal(model.loading, false)
})
test('reset and bulk revocation target configuration independently without removing the connection', async () => {
  const { model, calls } = fixture()
  await model.resetPolicy('write')
  await model.revokeAll()
  assert.deepEqual(calls.find((call) => call.method === 'resetMcpToolPolicy')?.body, { connectorId: 'connector', tool: 'write' })
  assert.deepEqual(calls.find((call) => call.method === 'revokeMcpToolCalls')?.body, { connectorId: 'connector' })
  assert.equal(calls.some((call) => call.method === 'removeConnector'), false)
})
test('a failed policy save retains existing settings and an actionable error', async () => {
  const { model } = fixture(async (method) => {
    if (method === 'setMcpToolPolicy') throw new Error('Disconnected')
    return { items: [], next: null }
  })
  model.policies = [{ tool: 'write', mode: 'deny' }]
  await model.setPolicy('write', 'allow')
  assert.equal(model.policies[0].mode, 'deny')
  assert.match(model.error ?? '', /Disconnected/)
  assert.equal(model.busy, false)
})
test('a second click cannot dispatch another management action while one is pending', async () => {
  const pending = deferred<unknown>()
  const { model, calls } = fixture(async (method) => method === 'setMcpToolPolicy' ? pending.promise : { items: [], next: null })
  const saving = model.setPolicy('write', 'deny')
  await model.revokeAll()
  pending.resolve({})
  await saving
  assert.equal(calls.filter((call) => call.method === 'setMcpToolPolicy').length, 1)
  assert.equal(calls.some((call) => call.method === 'revokeMcpToolCalls'), false)
})

test('quiet refresh keeps loaded later pages and their inspected request', async () => {
  const second = { ...row, id: 'second', tool: 'read' }
  const { model } = fixture(async (method, body) => {
    if (method === 'getMcpToolSettings') return { items: [] }
    if (method === 'getMcpToolCall') return { ...second, arguments: { exact: true } }
    return body.afterId ? { items: [second], next: null } : { items: [row], next: 'call' }
  })
  await model.load()
  await model.load(true)
  await model.inspect(second)
  await model.load()
  assert.deepEqual(model.requests.map((item) => item.id), ['call', 'second'])
  assert.equal(model.detail?.id, 'second')
  assert.deepEqual(model.detail?.arguments, { exact: true })
})

test('an old mutation completing after remount does not replace a newer notice or settings', async () => {
  const pending = deferred<unknown>()
  const { model } = fixture(async (method) => method === 'setMcpToolPolicy' ? pending.promise : { items: [], next: null })
  const saving = model.setPolicy('write', 'deny')
  model.dispose(); model.resume()
  await model.revokeAll()
  const notice = model.notice
  pending.resolve({})
  await saving
  assert.equal(model.notice, notice)
  assert.match(model.notice ?? '', /Current requests revoked/)
  assert.equal(model.busy, false)
})
test('saved result recovery is read-only and never repeats authorization or execution', async () => {
  const reply = { jsonrpc: '2.0', id: 1, result: { content: [{ type: 'text', text: 'saved output' }] } }
  const { model, calls } = fixture(async () => ({ status: 'dispatched', reply }))
  await model.retrieveResult('saved-call')
  assert.deepEqual(model.receipt?.reply, reply)
  assert.deepEqual(calls, [{ method: 'getMcpToolResult', body: { connectorId: 'connector', id: 'saved-call' } }])
})
test('a missing receipt stays explicitly unknown instead of rerunning the tool', async () => {
  const { model, calls } = fixture(async () => ({ status: 'dispatched', reply: null }))
  await model.retrieveResult('unknown-call')
  assert.equal(model.receipt?.reply, null)
  assert.equal(calls.length, 1)
})
