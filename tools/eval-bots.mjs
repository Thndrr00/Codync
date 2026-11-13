#!/usr/bin/env node
// Runs the bot evaluation cases against a running host, without any provider key of its own.
//
//   node tools/eval-bots.mjs [--home <CODYNC_HOME>] [--port 19223] [evals/*.json]
//
// A case file is an array (or {"cases": [...]}) of:
//   { "name": "answers where login lives", "botId": "b1", "prompt": "where is login handled?",
//     "expect": ["auth"], "expectTools": ["search_workspace"], "timeoutSeconds": 90 }
//
// `expect` matches the final reply case-insensitively; every string must appear. `expectTools` must
// appear among the tool entries of the turn. Exits 1 when any case fails.
import { readFileSync, readdirSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { homedir } from 'node:os'

const args = process.argv.slice(2)
const option = (name, fallback) => {
  const index = args.indexOf(name)
  if (index === -1) return fallback
  const value = args[index + 1]
  args.splice(index, 2)
  return value
}
const home = option('--home', process.env.CODYNC_HOME ?? join(process.env.LOCALAPPDATA ?? homedir(), 'Codync', 'dev'))
const port = Number(option('--port', process.env.CODYNC_PORT ?? 19223))
const botsOption = option('--bots', '').split(',').map((id) => id.trim()).filter(Boolean)
const files = args.length ? args : defaultFiles()

function defaultFiles() {
  try {
    return readdirSync('evals').filter((f) => f.endsWith('.json')).map((f) => join('evals', f))
  } catch {
    return []
  }
}

const token = readFileSync(join(home, 'token'), 'utf8').trim()
const call = async (method, body = {}) => {
  const res = await fetch(`http://127.0.0.1:${port}/api/${method}`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
  const answer = await res.json().catch(() => ({}))
  if (!res.ok) throw new Error(`${method}: HTTP ${res.status} ${JSON.stringify(answer)}`)
  return answer
}

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms))

async function runCase(test) {
  const timeout = (test.timeoutSeconds ?? 90) * 1000
  const started = Date.now()
  const nonce = crypto.randomUUID()
  await call('send', { botId: test.botId, text: test.prompt, clientNonce: nonce })
  let final = null
  let tools = []
  while (Date.now() - started < timeout) {
    await sleep(1000)
    const { entries } = await call('history', { botId: test.botId, beforeSeq: 0, limit: 200 })
    const ours = entries.filter((e) => (e.createdAt ?? 0) >= started)
    tools = ours.filter((e) => e.kind === 'tool').map((e) => e.data?.name).filter(Boolean)
    final = [...ours].reverse().find((e) => e.kind === 'agent' && e.data?.final === true && e.data?.text)
  }
  const text = final?.data?.text ?? ''
  const missing = (test.expect ?? []).filter((needle) => !text.toLowerCase().includes(String(needle).toLowerCase()))
  const missingTools = (test.expectTools ?? []).filter((name) => !tools.includes(name))
  return { name: test.name ?? test.prompt, ok: missing.length === 0 && missingTools.length === 0, text, missing, missingTools }
}

if (files.length === 0) {
  console.log('no eval files; pass one or put cases under evals/*.json')
  process.exit(0)
}

let failed = 0
for (const file of files) {
  const raw = JSON.parse(readFileSync(resolve(file), 'utf8'))
  const cases = Array.isArray(raw) ? raw : (raw.cases ?? [])
  for (const test of cases) {
    const bots = botsOption.length ? botsOption : [test.botId]
    for (const botId of bots) {
      const run = { ...test, botId }
      const label = bots.length > 1 ? `${run.name ?? run.prompt} [${botId}]` : (run.name ?? run.prompt)
      try {
        const result = await runCase(run)
        if (result.ok) {
          console.log(`ok    ${label}`)
        } else {
          failed += 1
          console.log(`FAIL  ${label}`)
          if (result.missing.length) console.log(`      missing: ${result.missing.join(', ')}`)
          if (result.missingTools.length) console.log(`      tools not called: ${result.missingTools.join(', ')}`)
          if (result.text) console.log(`      reply: ${result.text.slice(0, 240)}`)
        }
      } catch (error) {
        failed += 1
        console.log(`ERROR ${label}: ${error.message}`)
      }
    }
  }
}
console.log(failed ? `${failed} case(s) failed` : 'all cases passed')
process.exit(failed ? 1 : 0)
