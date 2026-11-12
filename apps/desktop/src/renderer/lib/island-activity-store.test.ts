import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { Usage } from '../../shared/models.ts'
import type { AppModel } from '../store/app-model.ts'
import type { FlowCanvas } from './routine-flow.ts'
import { IslandActivityStore } from '../store/island-activity-store.ts'
import { activeCall } from '../views/call/active-call.ts'

type Request = (method: string, body: Record<string, unknown>) => Promise<unknown>

test('refreshing a replacement computer cannot relabel cached runs and errors as its own', async () => {
  const original = fixture()
  await original.model.load()
  assert.ok(original.model.snapshot().runs.length > 0)
  original.model.error = 'Previous computer error'
  original.model.refreshError = 'Previous usage error'
  const replacement = fixture('replacement', async (method) => method === 'usage' ? { providers: [] } : { activity: [] })
  original.app.local = replacement.local
  assert.equal(original.model.snapshot().error, null)
  assert.equal(original.model.snapshot().refreshError, null)
  await original.model.perform({ kind: 'refreshUsage' })
  assert.deepEqual(original.model.snapshot().runs, [])
  assert.deepEqual(original.model.snapshot().channels, [])
  assert.equal(original.model.snapshot().error, null)
  original.model.dispose()
})

test('a delayed island command cannot mute or end a replacement call', async () => {
  const { model } = fixture()
  const invoked: string[] = []
  const release = activeCall.publish({ id: 'current', ref: 'computer/bot', name: 'Bot', phase: 'listening', muted: false }, { mute: () => invoked.push('mute'), end: () => invoked.push('end') })
  try {
    await model.perform({ kind: 'callMute', callId: 'previous' })
    await model.perform({ kind: 'callEnd', callId: 'previous' })
    assert.deepEqual(invoked, [])
    await model.perform({ kind: 'callMute', callId: 'current' })
    await model.perform({ kind: 'callEnd', callId: 'current' })
    assert.deepEqual(invoked, ['mute', 'end'])
  } finally { release(); model.dispose() }
})

function canvas(botId = 'bot'): FlowCanvas {
  return {
    botId, rev: 1, links: [], positions: [], placedBotIds: [],
    routines: [{
      routine: { id: 'routine', botId, name: 'Release', instruction: '', triggers: [], enabled: true, createdAt: 1, updatedAt: 1, triggerDescriptions: [] },
      upcomingRuns: [],
      recentRuns: [
        { id: 'latest', routineId: 'routine', botId, status: 'pending', createdAt: 200 },
        { id: 'earlier', routineId: 'routine', botId, status: 'succeeded', createdAt: 100 },
      ],
      steps: [{ id: 'step', runId: 'earlier', botId, deliveryId: null, input: 'Private input', output: 'Private output', status: 'succeeded', error: null, createdAt: 101, updatedAt: 102 }],
    }],
  }
}

function fixture(id = 'computer', request: Request = async (method) => method === 'routineFlowCanvas' ? canvas() : { activity: [] }) {
  const calls: { method: string; body: Record<string, unknown> }[] = []
  const stopped: string[] = []
  const local = {
    computer: { id, name: id }, connection: { kind: 'online' }, connectionLabel: 'Connected', hostRoute: 'local',
    health: null, lastError: null, usage: { providers: [] } as Usage,
    n8n: { enabled: true, state: 'running', error: null, mcp: true, auth: 'PRIVATE_COOKIE', url: 'http://private-editor', path: 'PRIVATE_PATH', port: 5678 },
    bots: new Map([['bot', { id: 'bot', name: 'Builder' }]]),
    ready: async () => ({ call: async (method: string, body: Record<string, unknown>) => { calls.push({ method, body }); return request(method, body) } }),
    stop: (botId: string) => { stopped.push(botId) },
  }
  const roster = [{ ref: { computerId: id, botId: 'bot' }, bot: { id: 'bot', name: 'Builder', kind: 'bot', unread: 0 }, store: local }]
  const app = { local, roster, store: (computerId: string) => computerId === id ? local : null }
  return { app, local, calls, stopped, model: new IslandActivityStore(app as unknown as AppModel) }
}

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason: Error) => void
  const promise = new Promise<T>((done, fail) => { resolve = done; reject = fail })
  return { promise, resolve, reject }
}

test('host run steps stay with their own run and channel activity contains only real mentions', async () => {
  const { model } = fixture('computer', async (method) => method === 'routineFlowCanvas' ? canvas() : {
    activity: [
      { id: 'mention', botId: 'bot', source: 'slack', channel: 'C1', mentioned: true, at: 300, private: 'PRIVATE_MESSAGE' },
      { id: 'ordinary', botId: 'bot', source: 'discord', channel: 'C2', mentioned: false, at: 301 },
    ],
  })
  await model.load()
  const activity = model.snapshot()
  assert.deepEqual(activity.runs.map((run) => [run.id, run.ref, run.steps]), [
    ['latest', 'computer/bot', []],
    ['earlier', 'computer/bot', [{ name: 'Builder', status: 'succeeded' }]],
  ])
  assert.deepEqual(activity.channels, [{ id: 'mention', ref: 'computer/bot', source: 'slack', channel: 'C1', at: 300 }])
  assert.equal(JSON.stringify(activity).includes('PRIVATE_MESSAGE'), false)
  assert.equal(JSON.stringify(activity).includes('Private input'), false)
  assert.equal(JSON.stringify(activity).includes('Private output'), false)
})

test('the island gets n8n presentation state without its credentials or local paths', () => {
  const { model } = fixture()
  const activity = model.snapshot()
  assert.deepEqual(activity.n8n, { state: 'running', error: null, mcp: true })
  const json = JSON.stringify(activity)
  for (const privateValue of ['PRIVATE_COOKIE', 'PRIVATE_PATH', 'http://private-editor', '5678']) assert.equal(json.includes(privateValue), false)
})

test('usage refresh errors are explicit and offline refresh never calls the host', async () => {
  const { model, local, calls } = fixture('computer', async () => { throw new Error('Usage probe failed') })
  await model.perform({ kind: 'refreshUsage' })
  assert.equal(model.snapshot().refreshError, 'Usage probe failed')
  assert.equal(model.snapshot().refreshing, false)
  assert.deepEqual(calls, [{ method: 'usage', body: { refresh: true } }])
  local.connection.kind = 'offline'
  await model.perform({ kind: 'refreshUsage' })
  assert.equal(model.snapshot().refreshError, 'Connect this computer to refresh usage.')
  assert.equal(calls.length, 1)
})

test('stop actions resolve the named computer and bot through existing store actions', async () => {
  const { model, stopped } = fixture()
  await model.perform({ kind: 'stopBot', ref: 'computer/bot' })
  await model.perform({ kind: 'stopBot', ref: 'other/bot' })
  await model.perform({ kind: 'stopBot', ref: 'malformed' })
  assert.deepEqual(stopped, ['bot'])
})

test('a slow old computer cannot populate the replacement computer or block its next load', async () => {
  const entered = deferred<void>()
  const result = deferred<FlowCanvas>()
  const old = fixture('old', async (method) => {
    if (method === 'routineFlowCanvas') { entered.resolve(); return result.promise }
    return { activity: [] }
  })
  const next = fixture('new')
  const reading = old.model.load()
  await entered.promise
  old.app.local = next.local
  old.app.roster = next.app.roster
  result.resolve(canvas())
  await reading
  assert.deepEqual(old.model.snapshot().runs, [])
  assert.deepEqual(old.model.snapshot().channels, [])
  await old.model.load()
  assert.equal(old.model.snapshot().runs[0]?.ref, 'new/bot')
  assert.ok(next.calls.some((call) => call.method === 'channelActivity'))
})

test('disposal invalidates outstanding activity reads and prevents later polling', async () => {
  const entered = deferred<void>()
  const result = deferred<FlowCanvas>()
  const { model, calls } = fixture('computer', async (method) => {
    if (method === 'routineFlowCanvas') { entered.resolve(); return result.promise }
    return { activity: [] }
  })
  const reading = model.load()
  await entered.promise
  model.dispose()
  result.resolve(canvas())
  await reading
  assert.deepEqual(model.snapshot().runs, [])
  await model.load()
  assert.equal(calls.length, 1)
})

test('an old connection failure is not shown as an error on a replacement computer', async () => {
  const old = fixture('old')
  const next = fixture('new')
  const ready = deferred<Awaited<ReturnType<typeof old.local.ready>>>()
  old.local.ready = () => ready.promise
  const reading = old.model.load()
  old.app.local = next.local
  old.app.roster = next.app.roster
  ready.reject(new Error('Old computer disconnected'))
  await reading
  assert.equal(old.model.snapshot().error, null)
})

test('a failed usage probe on an old computer cannot set the replacement computer error', async () => {
  const entered = deferred<void>()
  const answer = deferred<unknown>()
  const old = fixture('old', async () => { entered.resolve(); return answer.promise })
  const next = fixture('new')
  const refreshing = old.model.perform({ kind: 'refreshUsage' })
  await entered.promise
  old.app.local = next.local
  old.app.roster = next.app.roster
  answer.reject(new Error('Old usage probe failed'))
  await refreshing
  assert.equal(old.model.snapshot().refreshError, null)
})
