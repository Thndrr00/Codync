// Acceptance probe: an existing Windows Chrome/Edge, production fill function, dummy values only.
// Does not build/install Codync, read provider credentials, submit forms or touch normal browser profiles.
import {existsSync,readFileSync,mkdtempSync,realpathSync,rmSync} from 'node:fs'
import {join,resolve,dirname} from 'node:path'
import {tmpdir} from 'node:os'
import {spawn,execFileSync} from 'node:child_process'
import {setTimeout as delay} from 'node:timers/promises'
import assert from 'node:assert/strict'

const paths=['PROGRAMFILES','PROGRAMFILES(X86)','LOCALAPPDATA'].flatMap(key=>process.env[key]?
  ['Microsoft/Edge/Application/msedge.exe','Google/Chrome/Application/chrome.exe'].map(path=>join(process.env[key],path)):[])
const executable=paths.find(path=>path.endsWith('chrome.exe')&&existsSync(path))??paths.find(existsSync)
if(process.platform!=='win32'||!executable)throw Error('An existing Windows Chrome or Edge is required')
const parent=realpathSync(tmpdir())
const folder=mkdtempSync(join(parent,'codync-protected-probe-'))
const broker=readFileSync(new URL('../host/src/screen/protected-browser-broker.mjs',import.meta.url),'utf8')
const child=spawn(process.execPath,['-e',broker,'--',executable,folder,'--headless-probe'],{stdio:['pipe','pipe','ignore'],windowsHide:true})
const source=readFileSync(new URL('../host/src/screen/protected-fill.js',import.meta.url),'utf8')
let sequence=0
const pending=new Map()
const call=(method,params={},sessionId)=>new Promise((resolve,reject)=>{
  const id=++sequence
  const timer=setTimeout(()=>{pending.delete(id);reject(Error(`Timed out: ${method}`))},12000)
  pending.set(id,{resolve,reject,timer,method})
  child.stdin.write(JSON.stringify({id,method,params,...(sessionId?{sessionId}:{})})+'\n')
})
try {
  let buffered=''
  child.stdout.setEncoding('utf8')
  child.stdout.on('data',data=>{
    buffered+=data
    let newline
    while((newline=buffered.indexOf('\n'))>=0){
    const reply=JSON.parse(buffered.slice(0,newline));buffered=buffered.slice(newline+1)
    const request=pending.get(reply.id)
    if(!request)continue
    pending.delete(reply.id);clearTimeout(request.timer)
    if(reply.error||reply.result?.exceptionDetails)request.reject(Error(`CDP refused fixture ${request.method}: ${reply.error?.message ?? reply.result?.exceptionDetails?.text ?? 'unknown'}`))
    else request.resolve(reply.result)
    }
  })
  const startup=(await call('Target.getTargets')).targetInfos
  assert.equal(existsSync(join(folder,'DevToolsActivePort')),false,'A TCP debugging endpoint must not be created')
  const context=await call('Target.createBrowserContext',{disposeOnDetach:false})
  const created=await call('Target.createTarget',{url:'https://example.com',browserContextId:context.browserContextId,newWindow:true})
  for(const tab of startup.filter(tab=>tab.type==='page'))await call('Target.closeTarget',{targetId:tab.targetId})
  let target
  for(let n=0;n<100;n++) {
    target=(await call('Target.getTargets')).targetInfos.find(target=>target.targetId===created.targetId&&target.url==='https://example.com/')
    if(target)break
    await delay(100)
  }
  assert.ok(target?.browserContextId,'Expected a private HTTPS page context')
  const {sessionId}=await call('Target.attachToTarget',{targetId:target.targetId,flatten:true})
  const evaluate=async(expression)=>call('Runtime.evaluate',{expression,returnByValue:true},sessionId)
  for(let n=0;n<100;n++) {
    if((await evaluate("!!document.body && document.readyState === 'complete' && location.origin === 'https://example.com'")).result.value)break
    await delay(100)
  }
  await evaluate("document.body.innerHTML='<input id=secret type=password><input id=code autocomplete=one-time-code>'; document.getElementById('secret').focus()")
  const {result:{objectId}}=await call('Runtime.evaluate',{expression:"document.getElementById('secret')"},sessionId)
  const invoke=async(origin,kind,...secret)=>call('Runtime.callFunctionOn',{objectId,functionDeclaration:source,arguments:[origin,kind,...secret].map(value=>({value})),returnByValue:true},sessionId)
  assert.equal((await invoke('https://example.com','password')).result.value.ready,true)
  assert.equal((await invoke('https://wrong.example','password')).result.value.ready,false)
  assert.equal((await invoke('https://example.com','password','dummy-fixture-password')).result.value,'filled')
  await evaluate("document.getElementById('code').focus()")
  assert.equal((await invoke('https://example.com','password','dummy-stale')).result.value,'refused')
  const popup=await call('Target.createTarget',{url:'about:blank',browserContextId:target.browserContextId})
  const clean=await call('Target.createTarget',{url:'about:blank',browserContextId:target.browserContextId,newWindow:true})
  await call('Target.closeTarget',{targetId:target.targetId});await call('Target.closeTarget',{targetId:popup.targetId})
  let remaining
  for(let n=0;n<30;n++) {
    remaining=(await call('Target.getTargets')).targetInfos.filter(target=>target.type!=='browser')
    if(remaining.length===1&&remaining[0].targetId===clean.targetId)break
    await delay(100)
  }
  assert.ok(!remaining.some(tab=>tab.targetId===target.targetId||tab.targetId===popup.targetId))
  const blank=remaining.find(tab=>tab.targetId===clean.targetId)
  assert.ok(blank);assert.equal(blank.browserContextId,target.browserContextId)
  assert.equal(blank.url,'about:blank');assert.equal(blank.openerId,undefined)
  const preserve=remaining.every(tab=>tab.targetId===clean.targetId)
  if(!preserve)await call('Browser.close')
  assert.equal(existsSync(join(folder,'DevToolsActivePort')),false,'A TCP debugging endpoint must not be created')
  console.log(JSON.stringify({status:'passed',sessionPreserved:preserve,fallback:preserve?null:'unverified targets require closing the entire owned browser',checks:['inherited pipes, no TCP debugger','real Chromium protected password input','exact origin rejection','stale focus rejection','private popup and original page destruction','clean opener-free page in same private context','unverified-target closure fallback'],scope:'production JavaScript + CDP contract; no compiled Codync integration tested'}))
} finally {
  child.stdin.end()
  for(const request of pending.values()) {clearTimeout(request.timer);request.reject(Error('Probe closed'))}
  if(child.pid)try{execFileSync('taskkill.exe',['/PID',String(child.pid),'/T','/F'],{stdio:'ignore',windowsHide:true})}catch{}
  execFileSync('powershell.exe',['-NoProfile','-Command',"$folder=$env:CODYNC_PROBE_PROFILE; $browserDir=[IO.Path]::GetDirectoryName($env:CODYNC_PROBE_BROWSER); Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -and $_.CommandLine.Contains($folder) -and $_.ExecutablePath -and $_.ExecutablePath.StartsWith($browserDir+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase) } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }"],{stdio:'ignore',windowsHide:true,env:{...process.env,CODYNC_PROBE_PROFILE:folder,CODYNC_PROBE_BROWSER:executable}})
  await delay(500)
  const checked=resolve(folder)
  assert.equal(dirname(checked),parent)
  assert.ok(checked.startsWith(join(parent,'codync-protected-probe-')))
  try { rmSync(checked,{recursive:true,force:true,maxRetries:4,retryDelay:250}) }
  catch { console.warn(`Probe process stopped; Windows retained temporary profile: ${checked}`) }
}
