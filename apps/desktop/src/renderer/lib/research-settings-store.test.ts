import { test } from 'node:test'
import assert from 'node:assert/strict'
import type { HostClient } from '../client/host-client'
import { ResearchSettingsStore } from '../store/research-settings-store.ts'
function deferred<T>() { let resolve!: (value:T)=>void; const promise=new Promise<T>(done=>{resolve=done});return {promise,resolve} }
function fixture(handle:(method:string,body:Record<string,unknown>)=>Promise<unknown>) {
  const calls:{method:string;body:Record<string,unknown>}[]=[]
  const client={call:async(method:string,body:Record<string,unknown>)=>{calls.push({method,body});return handle(method,body)}} as unknown as HostClient
  return {model:new ResearchSettingsStore(async()=>client),calls}
}
test('research credentials are sent to the host once and the model retains status only',async()=>{
  const {model,calls}=fixture(async(method,body)=>({configured:method==='researchStatus'?false:!!body.key}))
  await model.load();assert.equal(model.configured,false)
  await model.save('  fixture-key  ')
  assert.deepEqual(calls[1],{method:'setResearchKey',body:{key:'fixture-key'}})
  assert.equal(model.configured,true);assert.equal(JSON.stringify(model).includes('fixture-key'),false)
  assert.match(model.notice!,/validity is checked when you research/)
  await model.save('');assert.equal(model.configured,false)
})
test('a credential save supersedes an older status load',async()=>{
  const stale=deferred<unknown>()
  const {model}=fixture(async(method)=>method==='researchStatus'?stale.promise:{configured:true})
  const load=model.load();await Promise.resolve();await model.save('fixture-key')
  stale.resolve({configured:false});await load
  assert.equal(model.configured,true);assert.equal(model.loading,false)
})
test('duplicate clicks do not repeat a credential mutation',async()=>{
  const saved=deferred<unknown>()
  const {model,calls}=fixture(async()=>saved.promise)
  const first=model.save('fixture-key');await model.save('other-key')
  saved.resolve({configured:true});await first
  assert.equal(calls.length,1)
})
test('an old save cannot overwrite a refreshed panel after remount',async()=>{
  const saved=deferred<unknown>()
  const {model}=fixture(async(method)=>method==='setResearchKey'?saved.promise:{configured:false})
  const first=model.save('fixture-key');await Promise.resolve();model.dispose();model.resume();await model.load()
  saved.resolve({configured:true});await first
  assert.equal(model.configured,false);assert.equal(model.notice,null)
})
test('host failures remain visible without claiming the key was validated',async()=>{
  const {model}=fixture(async()=>{throw new Error('Credential vault unavailable')})
  await model.save('fixture-key')
  assert.match(model.error!,/Credential vault unavailable/);assert.equal(model.configured,null);assert.equal(model.busy,false)
})
