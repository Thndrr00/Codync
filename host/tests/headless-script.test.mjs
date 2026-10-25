import { test } from 'node:test'
import assert from 'node:assert/strict'
import { createServer } from 'node:http'
import { spawn } from 'node:child_process'
import { mkdtempSync, writeFileSync, rmSync, rmdirSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

async function run(handle, replyTo, timeout='1') {
  const requests=[]
  const server=createServer(async(req,res)=>{
    let raw=''; for await(const chunk of req) raw+=chunk
    const body=JSON.parse(raw); requests.push([req.url,body])
    assert.equal(req.headers.authorization,'Bearer fixture-token')
    const result=handle(req.url,body)
    res.writeHead(result.status??200,{'content-type':'application/json'});res.end(JSON.stringify(result.body))
  })
  await new Promise(done=>server.listen(0,'127.0.0.1',done))
  const directory=mkdtempSync(join(tmpdir(),'codync-headless-'))
  const token=join(directory,'token'), prompt=join(directory,'prompt.txt')
  writeFileSync(token,'fixture-token');writeFileSync(prompt,'Explain the task')
  try {
    const args=[resolve('tools/run-headless.mjs'),'--token-file',token,
      '--port',String(server.address().port),'--bot','bot','--prompt-file',prompt,'--timeout',timeout]
    if(replyTo) args.push('--reply-to',replyTo)
    const child=spawn(process.execPath,args,{windowsHide:true})
    child.stdin.end()
    const timer=setTimeout(()=>child.kill(),15_000)
    let stdout='',stderr='';child.stdout.on('data',b=>stdout+=b);child.stderr.on('data',b=>stderr+=b)
    const exit=await new Promise((done,reject)=>{child.on('error',reject);child.on('exit',done)})
    clearTimeout(timer)
    return {exit,stdout,stderr,requests}
  } finally {server.closeAllConnections(); await new Promise(done=>server.close(done));rmSync(token);rmSync(prompt);rmdirSync(directory) }
}
const entry={id:'sent',seq:10,turn:3,data:{status:'sent'}}
const completed={id:'sent',botId:'bot',threadId:null,status:'sent',result:{version:1,state:'done',text:'answer'}}
test('headless runner sends once and returns the matching final result', async()=>{
  const result=await run(path=>({body:path.endsWith('/send')?{entry}:completed}))
  assert.equal(result.exit,0,result.stderr)
  assert.equal(JSON.parse(result.stdout).id,'sent')
  assert.equal(JSON.parse(result.stdout).text,'answer')
  assert.equal(result.requests.filter(([path])=>path.endsWith('/send')).length,1)
})
test('headless uses an exact message lookup independently of history pagination', async()=>{
  const result=await run((path,body)=>{
    if(path.endsWith('/send')) {assert.equal(body.replyTo,'root');return {body:{entry}}}
    assert.equal(path,'/api/messageResult');assert.deepEqual(body,{botId:'bot',id:'sent'})
    return {body:completed}
  },'root')
  assert.equal(result.exit,0,result.stderr)
})
test('an uncertain send is never retried', async()=>{
  const result=await run(()=>({status:500,body:{error:'lost response'}}))
  assert.notEqual(result.exit,0)
  assert.match(result.stderr,/Send outcome unknown/)
  assert.equal(result.requests.length,1)
})
test('a later unrelated final answer cannot complete the requested message',async()=>{
  const result=await run(path=>({body:path.endsWith('/send')?{entry}:{...completed,result:null,entries:[entry,
    {id:'later',seq:11,turn:4,kind:'user',data:{turnResult:{version:1,state:'done',text:'other answer'}}},
    {id:'unrelated',seq:12,turn:4,kind:'agent',threadId:null,data:{final:true,text:'other answer'}}]}}))
  assert.notEqual(result.exit,0);assert.match(result.stderr,/Timed out waiting for message sent/)
  assert.equal(result.stdout,'');assert.equal(result.requests.filter(([path])=>path.endsWith('/send')).length,1)
})
test('waiting for owner sign-in is nonterminal and can complete after authorized resume',async()=>{
  let reads=0
  const result=await run(path=>({body:path.endsWith('/send')?{entry}:(++reads===1
    ? {...completed,result:{version:1,state:'waitingForSignIn',text:null}}:completed)}),undefined,'3')
  assert.equal(result.exit,0,result.stderr);assert.equal(JSON.parse(result.stdout).text,'answer');assert.equal(reads,2)
})
test('host restart is reported as interrupted instead of repeating the message',async()=>{
  const result=await run(path=>({body:path.endsWith('/send')?{entry}:{...completed,result:{version:1,state:'interrupted',text:null}}}))
  assert.notEqual(result.exit,0);assert.match(result.stderr,/interrupted/)
  assert.equal(result.requests.filter(([path])=>path.endsWith('/send')).length,1)
})
