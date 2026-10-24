import {test} from 'node:test'
import assert from 'node:assert/strict'
import {VoiceDecisionsStore} from '../store/voice-decisions-store.ts'
import type {HostClient} from '../client/host-client'
function fixture(handler: (method: string, body: unknown) => Promise<unknown>) {
  const calls: {method: string; body: unknown}[]=[]
  const client={call:async(method:string,body:unknown)=>{calls.push({method,body});return handler(method,body)}} as unknown as HostClient
  return {model:new VoiceDecisionsStore(async()=>client,'bot','receipt'),calls}
}
test('only an explicit confirmation sends the exact owner-written summary',async()=>{
  const {model,calls}=fixture(async()=>({entry:null}))
  await model.load();model.setSummary('  Use the blue design.  ')
  assert.equal(calls.length,1)
  await model.confirm();await model.confirm()
  assert.deepEqual(calls[1],{method:'confirmVoiceDecisions',body:{botId:'bot',receiptId:'receipt',summary:'Use the blue design.'}})
  assert.equal(calls.length,2);assert.equal(model.confirmed,'Use the blue design.')
})
test('an existing immutable decision receipt prevents another confirmation',async()=>{
  const {model,calls}=fixture(async()=>({entry:{data:{voiceDecisions:{summary:'Already confirmed'}}}}))
  await model.load();model.setSummary('replace');await model.confirm()
  assert.equal(calls.length,1);assert.equal(model.confirmed,'Already confirmed')
})
test('uncertain confirmation stays an error and is never automatically retried',async()=>{
  const {model,calls}=fixture(async(method)=>{if(method==='voiceDecisions')return{entry:null};throw new Error('Outcome unknown')})
  await model.load();model.setSummary('Decision');await model.confirm()
  assert.equal(model.confirmed,null);assert.match(model.error??'',/unknown/);assert.equal(calls.length,2)
})
test('closing during a request ignores late replies and loading must finish before confirmation',async()=>{
  let done!:(value:unknown)=>void
  const {model,calls}=fixture(async()=>new Promise(resolve=>{done=resolve}))
  model.setSummary('Decision');const loading=model.load();await Promise.resolve();await Promise.resolve()
  await model.confirm();assert.equal(calls.length,1)
  model.dispose();done({entry:{data:{voiceDecisions:{summary:'late'}}}});await loading
  assert.equal(model.confirmed,null)
})

test('closing while acquiring the client cancels confirmation before dispatch',async()=>{
  let release!:(client:HostClient)=>void
  let wait=false
  const calls:string[]=[]
  const client={call:async(method:string)=>{calls.push(method);return{entry:null}}} as unknown as HostClient
  const model=new VoiceDecisionsStore(()=>wait?new Promise(resolve=>{release=resolve}):Promise.resolve(client),'bot','receipt')
  await model.load();model.setSummary('Decision');wait=true
  const saving=model.confirm();model.dispose();release(client);await saving
  assert.deepEqual(calls,['voiceDecisions']);assert.equal(model.busy,false)
})

test('reopening during confirmation preserves its lock and reports an uncertain late outcome',async()=>{
  let release!:(value:unknown)=>void
  const {model,calls}=fixture(async(method)=>method==='voiceDecisions'?{entry:null}:new Promise(resolve=>{release=resolve}))
  await model.load();model.setSummary('Decision')
  const saving=model.confirm();await Promise.resolve();await Promise.resolve()
  model.dispose();await model.load();await model.confirm()
  assert.equal(model.busy,true);assert.equal(calls.length,2)
  release({entry:{}});await saving
  assert.equal(model.busy,false);assert.equal(model.confirmed,null);assert.match(model.error??'',/unknown/)
})

test('loading during confirmation does not invalidate its successful result',async()=>{
  let release!:(value:unknown)=>void
  const {model,calls}=fixture(async(method)=>method==='voiceDecisions'?{entry:null}:new Promise(resolve=>{release=resolve}))
  await model.load();model.setSummary('Decision')
  const saving=model.confirm();await Promise.resolve();await Promise.resolve();await model.load()
  release({entry:{}});await saving
  assert.equal(model.confirmed,'Decision');assert.equal(model.busy,false);assert.equal(calls.length,2)
})
