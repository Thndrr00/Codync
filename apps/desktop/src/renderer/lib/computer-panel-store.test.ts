import { test } from 'node:test'
import assert from 'node:assert/strict'
import type { HostClient } from '../client/host-client'
import { ComputerPanelStore, type ComputerPanelState } from '../store/computer-panel-store.ts'

const state: ComputerPanelState = {
  enabled: true, permissions: { browser: true, files: true, shell: true }, workspace: 'workspace',
  screen: { userControl: false, computerUse: true }, activity: [],
}

test('background polling cannot publish permissions from the middle of a mutation',async()=>{
  const saved=deferred<unknown>()
  const revoked={...state,permissions:{...state.permissions,files:false}}
  const {model,calls}=fixture(async(method)=>method==='updateBot'?saved.promise:structuredClone(revoked))
  const mutation=model.setPermissions(true,revoked.permissions)
  await Promise.resolve();await model.load()
  assert.equal(calls.filter(call=>call.method==='computerPanel').length,0)
  saved.resolve({});await mutation
  assert.equal(calls.filter(call=>call.method==='computerPanel').length,1)
  assert.equal(model.state?.permissions.files,false)
})
function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>((done) => { resolve = done }); return { promise, resolve } }
test('protected fills send only the immutable entry and owner value and retain no secret in the model',async()=>{
  const {model,calls}=fixture(async()=>({filled:true}))
  await model.protectedBrowser('fillProtectedCode','entry-123','123456')
  const sent=calls.find(call=>call.method==='fillProtectedCode')
  assert.deepEqual(sent?.body,{id:'entry-123',code:'123456'})
  assert.equal(JSON.stringify(model).includes('123456'),false)
})
test('handback distinguishes a preserved session from conservative whole-browser closure',async()=>{
  const {model,calls}=fixture(async(method)=>method==='finishProtectedEntry'?{browserSession:'closed'}:structuredClone(state))
  await model.protectedEntry(null,'entry-123',true)
  assert.deepEqual(calls.find(call=>call.method==='finishProtectedEntry')?.body,{id:'entry-123',keepSession:true})
  assert.match(model.notice??'',/discarded/)
})
function fixture(handle?: (method: string, body: Record<string, unknown>) => Promise<unknown>) {
  const calls: { method: string; body: Record<string, unknown>; timeout?: number }[] = []
  const client = { call: async (method: string, body: Record<string, unknown>, timeout?: number) => {
    calls.push({ method, body, timeout })
    if (handle) return handle(method, body)
    return method === 'computerPanel' ? structuredClone(state) : {}
  } } as unknown as HostClient
  return { model: new ComputerPanelStore('bot', async () => client), calls }
}
test('panel loads actual host state and permission changes are sent as host intents', async () => {
  const { model, calls } = fixture()
  await model.load()
  assert.deepEqual(model.state, state)
  await model.setPermissions(true, { ...state.permissions, shell: false })
  assert.deepEqual(calls.find((call) => call.method === 'updateBot')?.body,
    { id: 'bot', computer: true, computerPermissions: { browser: true, files: true, shell: false } })
})
test('later folder selection wins over an older delayed listing', async () => {
  const old = deferred<unknown>()
  const { model } = fixture(async (_, body) => body.path === 'old' ? old.promise : { items: [{ name: 'new', directory: false, link: false }], truncated: false })
  const first = model.open('old')
  await model.open('new')
  old.resolve({ items: [{ name: 'stale' }], truncated: false })
  await first
  assert.equal(model.path, 'new')
  assert.equal(model.files[0].name, 'new')
})
test('revoked file capability clears preview and ignores the in-flight file result', async () => {
  const file = deferred<unknown>()
  const { model } = fixture(async (method) => method === 'readComputerFile' ? file.promise : { ...state, permissions: { ...state.permissions, files: false } })
  const reading = model.open('note.txt', false)
  await model.load()
  file.resolve({ text: 'late contents' })
  await reading
  assert.equal(model.fileText, null)
  assert.deepEqual(model.files, [])
})
test('a running command prevents duplicate clicks and has a bounded request timeout', async () => {
  const pending = deferred<unknown>()
  const { model, calls } = fixture(async (method) => method === 'runComputerCommand' ? pending.promise : state)
  const first = model.run('echo hello')
  await model.run('echo duplicate')
  pending.resolve({ stdout: 'hello', stderr: '', exitCode: 0, success: true })
  await first
  assert.equal(calls.filter((call) => call.method === 'runComputerCommand').length, 1)
  assert.equal(calls.find((call) => call.method === 'runComputerCommand')?.timeout, 40_000)
  assert.equal(model.output?.stdout, 'hello')
})
test('command failure remains visible after background refresh instead of suggesting success', async () => {
  const { model } = fixture(async (method) => {
    if (method === 'runComputerCommand') throw new Error('Shell permission revoked')
    return state
  })
  await model.run('command')
  await model.load()
  assert.match(model.error ?? '', /Shell permission revoked/)
  assert.equal(model.output, null)
  assert.equal(model.running, false)
})
test('manual entry sends only public site metadata and finishes by immutable entry identity', async () => {
  const { model, calls } = fixture()
  await model.protectedEntry('https://example.com/login')
  await model.protectedEntry(null, 'entry-id')
  assert.deepEqual(calls.find((call) => call.method === 'beginProtectedEntry')?.body, { botId: 'bot', site: 'https://example.com/login' })
  assert.deepEqual(calls.find((call) => call.method === 'finishProtectedEntry')?.body, { id: 'entry-id' })
})
test('a completed command from an old panel cannot overwrite output after remount', async () => {
  const old = deferred<unknown>()
  const { model } = fixture(async (method, body) => method === 'runComputerCommand'
    ? body.command === 'old' ? old.promise : { stdout: 'new', stderr: '', exitCode: 0, success: true } : state)
  const running = model.run('old')
  model.dispose(); model.resume()
  await model.run('new')
  old.resolve({ stdout: 'old', stderr: '', exitCode: 0, success: true })
  await running
  assert.equal(model.output?.stdout, 'new')
})
test('a permission mutation supersedes an older background state response', async () => {
  const old = deferred<unknown>()
  let first = true
  const current = { ...state, permissions: { ...state.permissions, shell: false } }
  const { model } = fixture(async (method) => {
    if (method === 'computerPanel' && first) { first = false; return old.promise }
    return method === 'computerPanel' ? current : {}
  })
  const loading = model.load()
  await model.setPermissions(true, current.permissions)
  old.resolve(state)
  await loading
  assert.equal(model.state?.permissions.shell, false)
  assert.equal(model.loading, false)
})
