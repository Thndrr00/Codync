import {test} from 'node:test'
import assert from 'node:assert/strict'
import {OnboardingTracker} from './onboarding-tracker.ts'
function storage(){const map=new Map<string,string>();return{getItem:(key:string)=>map.get(key)??null,setItem:(key:string,value:string)=>{map.set(key,value)},map}}
test('offline steps survive reopening and an unfinished step records abandonment',async()=>{
  const local=storage();const delivered:unknown[]=[]
  const first=new OnboardingTracker(local,async()=>{throw Error('offline')})
  first.step('welcome');await first.flush()
  const next=new OnboardingTracker(local,async(event)=>{delivered.push(event)})
  next.recover();await Promise.resolve();await next.flush();await new Promise(resolve=>setImmediate(resolve))
  assert.deepEqual(delivered.map((event:any)=>[event.event,event.step]),[['onboarding_step_viewed','welcome'],['onboarding_abandoned','welcome']])
})
test('unknown delivery reuses its event identity instead of creating another event',async()=>{
  const local=storage();const ids:string[]=[];let fail=true
  const model=new OnboardingTracker(local,async(event)=>{ids.push(event.clientEventId);if(fail)throw Error('unknown')})
  model.step('pairing');await new Promise(resolve=>setImmediate(resolve));fail=false;await model.flush()
  assert.equal(ids.length,2);assert.equal(ids[0],ids[1])
})
test('late acknowledgements preserve events recorded while a batch is pending',async()=>{
  const local=storage();let release!:(value:unknown)=>void
  const model=new OnboardingTracker(local,async()=>new Promise(resolve=>{release=resolve}))
  model.step('welcome');await Promise.resolve();model.step('computer_access')
  release({});await new Promise(resolve=>setImmediate(resolve))
  const pending=JSON.parse([...local.map.values()][0]!).pending
  assert.equal(pending.length,1);assert.equal(pending[0].step,'computer_access')
})
test('opt-out clears persisted work and does not retain subsequent steps',async()=>{
  const local=storage();let enabled=true
  const model=new OnboardingTracker(local,async()=>{throw Error('offline')},()=>enabled)
  model.step('account');await new Promise(resolve=>setImmediate(resolve));enabled=false
  model.abandon();model.step('pairing');await model.flush()
  assert.deepEqual(JSON.parse([...local.map.values()][0]!),{active:null,pending:[]})
})
