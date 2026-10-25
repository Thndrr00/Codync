#!/usr/bin/env node
// One send, then read-only result polling. No compiler or desktop process is needed.
import { readFile } from 'node:fs/promises'
import { randomUUID } from 'node:crypto'
import { parseArgs } from 'node:util'
import { setTimeout as sleep } from 'node:timers/promises'

async function main() {
  const { values: args } = parseArgs({ options: {
    'token-file': { type: 'string' }, port: { type: 'string' }, bot: { type: 'string' },
    'prompt-file': { type: 'string' }, 'reply-to': { type: 'string' }, timeout: { type: 'string', default: '600' },
  } })
  const port = Number(args.port), timeout = Number(args.timeout)
  if (!args['token-file'] || !args.bot || !args['prompt-file'] || !Number.isInteger(port) || port<1 || port>65535
    || !Number.isFinite(timeout) || timeout<1 || timeout>86400) {
    throw new Error('Use --token-file PATH --port PORT --bot ID --prompt-file PATH [--reply-to ENTRY-ID] [--timeout SECONDS].')
  }
  const token = (await readFile(args['token-file'], 'utf8')).trim()
  if (!token) throw new Error('The host token file is empty.')
  const text = await readFile(args['prompt-file'], 'utf8')
  if (!text.trim()) throw new Error('The prompt file is empty.')
  async function call(method, body) {
    const response = await fetch(`http://127.0.0.1:${port}/api/${method}`, {
      method: 'POST', redirect: 'error', headers: { authorization: `Bearer ${token}`, 'content-type': 'application/json' },
      body: JSON.stringify(body), signal: AbortSignal.timeout(30_000),
    })
    if (!response.ok) throw new Error(`The host refused ${method} (${response.status}).`)
    return response.json()
  }
  const nonce = randomUUID(), replyTo = args['reply-to'] || null
  let sent
  try { sent = await call('send', { botId: args.bot, text, clientNonce: nonce, ...(replyTo ? { replyTo } : {}) }) }
  catch { throw new Error(`Send outcome unknown for client nonce ${nonce}. Inspect the conversation before retrying.`) }
  const entry = sent.entry
  if (!entry || typeof entry.id!=='string' || !Number.isInteger(entry.seq) || !Number.isInteger(entry.turn)) {
    throw new Error(`No valid saved message returned for nonce ${nonce}; do not resend automatically.`)
  }
  const deadline = Date.now()+timeout*1000
  while (Date.now()<deadline) {
    const message=await call('messageResult',{botId:args.bot,id:entry.id})
    if (message.id!==entry.id || message.botId!==args.bot || (message.threadId??null)!==null) {
      throw new Error(`Result identity changed; inspect message ${entry.id}.`)
    }
    if (message.status==='cancelled') throw new Error(`Message ${entry.id} was cancelled.`)
    if (message.status==='failed' && (!message.result || message.result.state==='waitingForSignIn')) throw new Error(`Message ${entry.id} was interrupted or failed without a durable result. Inspect before retrying.`)
    const result=message.result
    if (result && result.state!=='waitingForSignIn') {
      if (result.version!==1 || result.state!=='done' || typeof result.text!=='string' || !result.text.trim()) {
        throw new Error(`Message ${entry.id} finished without a successful text result (${result.state??'unknown'}).`)
      }
      process.stdout.write(`${JSON.stringify({id:entry.id,botId:args.bot,replyTo,...result})}\n`); return
    }
    await sleep(Math.min(1000,Math.max(1,deadline-Date.now())))
  }
  throw new Error(`Timed out waiting for message ${entry.id}. The host may still be working; no request was repeated or cancelled.`)
}
main().catch(error=>{process.stderr.write(`${error.message}\n`);process.exitCode=1})
