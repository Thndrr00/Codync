// Exercise an installed Windows host with disposable local data. No production state is used.
import assert from 'node:assert/strict'
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { setTimeout as delay } from 'node:timers/promises'

const [homeArg, portArg = '19480'] = process.argv.slice(2)
assert.ok(homeArg, 'Usage: node tools/verify-windows-install.mjs <test-home> [port]')
const home = resolve(homeArg)
const base = `http://127.0.0.1:${Number(portArg)}`
const token = readFileSync(join(home, 'token'), 'utf8').trim()
const results = []
const headers = { authorization: `Bearer ${token}`, 'content-type': 'application/json' }
async function api(method, body = {}, expected = 200) {
  const response = await fetch(`${base}/api/${method}`, {
    method: 'POST', headers, body: JSON.stringify(body), signal: AbortSignal.timeout(45000),
  })
  const text = await response.text()
  assert.equal(response.status, expected, `${method}: ${text.slice(0, 300)}`)
  return text ? JSON.parse(text) : null
}
async function check(name, run) {
  try { await run(); results.push({ name, result: 'pass' }); console.log(`PASS ${name}`) }
  catch (error) { results.push({ name, result: 'fail', error: error.message }); console.error(`FAIL ${name}: ${error.message}`) }
}
await api('setAnalytics', { enabled: false })
let bot, catalog, app, viewLink
const workspace = join(home, 'workspace')
mkdirSync(join(workspace, 'apps', 'windows-check'), { recursive: true })
writeFileSync(join(workspace, 'apps', 'windows-check', 'index.html'), '<!doctype html><title>Windows mini-app check</title><h1>WINDOWS_APP_OK</h1>')
await check('installed host health and development identity', async () => {
  const health = await (await fetch(`${base}/health`)).json()
  assert.equal(health.ok, true)
  assert.equal(health.environment, 'dev')
  const hello = await api('hello')
  assert.ok(hello.version)
})
await check('bot catalog contains bots and coordinated teams', async () => {
  catalog = await api('botCatalog')
  assert.ok(catalog.bots.length > 0)
  assert.ok(catalog.teams.length > 0)
})
await check('create bot preserves explicit agent and instructions', async () => {
  const created = await api('createBot', {
    name: 'Windows validation', description: 'Only local validation fixtures.', backend: 'codex', cwd: workspace,
    connectors: [], skills: [], permission: 'ask',
  })
  bot = created.bot
  assert.equal(bot.backend, 'codex')
  assert.equal(bot.description, 'Only local validation fixtures.')
})
await check('team installation creates a coordinator-led group', async () => {
  const installed = await api('installTeam', { id: catalog.teams[0].id, backend: 'codex' })
  assert.equal(installed.bot.kind, 'group')
  assert.ok(installed.bot.members.length >= 2)
  assert.ok(installed.bot.members.includes(installed.bot.chiefBotId))
})
await check('diagnostics and budgets respond for new bots', async () => {
  assert.ok(await api('doctor'))
  assert.ok(await api('setTokenBudget', { botId: bot.id, limit: 100000 }))
  assert.ok(await api('tokenBudgets'))
})
await check('portable bot export and import round trip', async () => {
  const bundle = await api('exportBot', { botId: bot.id })
  assert.equal(bundle.format, 'codync-bot-bundle')
  const imported = await api('importBots', { bundle, cwd: workspace })
  assert.equal(imported.bots.length, 1)
  assert.equal(imported.bots[0].backend, bot.backend)
})
await check('invalid multi-bot bundle imports atomically', async () => {
  const before = (await api('doctor')).checked
  const bundle = { format: 'codync-bot-bundle', version: 1, bots: [{ name: 'Atomic good', backend: 'codex' }, { name: 'Atomic bad', backend: 'unknown-windows-fixture' }] }
  await api('importBots', { bundle, cwd: workspace }, 400)
  assert.equal((await api('doctor')).checked, before)
})
await check('import refuses embedded executable fallback commands', async () => {
  const bundle = { format: 'codync-bot-bundle', version: 1, bots: [{ name: 'Rejected', backend: 'codex', fallbacks: [{ backend: 'custom', command: 'never-execute-this' }] }] }
  await api('importBots', { bundle, cwd: workspace }, 400)
})
await check('saved searches persist query and scope', async () => {
  const saved = await api('saveSearch', { query: 'Windows validation', scope: 'all' })
  assert.ok(saved.searches.some(search => search.query === 'Windows validation'))
  assert.ok((await api('savedSearches')).searches.length)
})
await check('all calendar and interval schedules compile in the host', async () => {
  for (const calendarStyle of ['minutes', 'hourly', 'daily', 'weekdays', 'weekly', 'days', 'monthly', 'custom']) {
    const draft = { kind: 'cron', amount: '5', unit: 60, calendarStyle, weekday: 1, selectedDays: [1, 3, 5], monthDay: 15, minuteStep: 15, hour: 9, minute: 0, expression: '0 9 * * 1-5', zone: 'UTC', original: [] }
    assert.equal((await api('routineSchedule', { draft })).triggers[0].type, 'cron', calendarStyle)
  }
  const preview = await api('routineSchedule', { draft: { kind: 'interval', amount: '5', unit: 60 } })
  assert.equal(preview.triggers[0].seconds, 300)
})
const chat = (name, args = {}) => api('chatCall', { botId: bot.id, name, arguments: args })
await check('bot mini-app registration, static page and helper serve', async () => {
  app = await chat('save_app', { slug: 'windows-check', title: 'Windows check' })
  const page = await fetch(app.url)
  assert.equal(page.status, 200)
  assert.match(await page.text(), /WINDOWS_APP_OK/)
  assert.match(page.headers.get('content-security-policy'), /sandbox/)
  assert.doesNotMatch(page.headers.get('content-security-policy'), /allow-same-origin/)
  assert.equal((await fetch(`${app.url}_codync.js`)).status, 200)
  assert.equal((await api('listBotApps', { botId: bot.id })).apps.length, 1)
})
await check('mini-app collections persist local JSON', async () => {
  const saved = await chat('app_data_put', { slug: 'windows-check', collection: 'notes', value: { text: 'WINDOWS_DATA_OK' } })
  assert.ok(saved.item.id)
  const rows = await chat('app_data_list', { slug: 'windows-check', collection: 'notes' })
  assert.equal(rows.items.length, 1)
})
await check('mini-app live channel delivers collection changes', async () => {
  const socket = new WebSocket(`${app.url.replace(/^http/, 'ws')}_live`)
  try {
    await new Promise((resolve, reject) => { socket.onopen = resolve; socket.onerror = reject; setTimeout(() => reject(Error('Live channel did not open')), 5000).unref() })
    const event = new Promise((resolve, reject) => {
      socket.onmessage = message => resolve(JSON.parse(message.data))
      setTimeout(() => reject(Error('Live update not received')), 5000).unref()
    })
    await chat('app_data_put', { slug: 'windows-check', collection: 'notes', id: 'live-check', value: { text: 'LIVE_OK' } })
    assert.equal((await event).type, 'change')
  } finally { socket.close() }
})
await check('mini-app view links can be revoked', async () => {
  viewLink = await api('createAppLink', { botId: bot.id, slug: 'windows-check', mode: 'view', label: 'Validation only' })
  assert.equal((await fetch(viewLink.url)).status, 200)
  await api('revokeAppLink', { botId: bot.id, slug: 'windows-check', token: viewLink.link.token })
  assert.equal((await fetch(viewLink.url)).status, 403)
})
await check('tracked open items update and resolve', async () => {
  const tracked = await chat('track_item', { title: 'Windows local check', checkAfterHours: 1 })
  const id = tracked.item?.id ?? tracked.id
  assert.ok(id)
  await chat('update_item', { id, note: 'Verified' })
  await chat('resolve_item', { id, status: 'resolved' })
})
await check('Windows shell jobs start and drain large stdout and stderr', async () => {
  const started = await chat('shell_job', { action: 'start', command: 'node -e "process.stdout.write(\'o\'.repeat(262144));process.stderr.write(\'e\'.repeat(262144))"' })
  assert.ok(started.jobId)
  let status
  for (let attempt = 0; attempt < 60; attempt++) {
    status = await chat('shell_job', { action: 'status', jobId: started.jobId })
    if (status.status === 'completed') break
    await delay(500)
  }
  assert.equal(status.status, 'completed')
  assert.equal(status.exitCode, 0)
  assert.ok(status.output.includes('o') && status.output.includes('e'))
  assert.ok(status.output.length <= 131072)
})
await check('stop all and failed-run cleanup accept an idle workspace', async () => {
  assert.ok(await api('stopAll'))
  assert.ok(await api('clearFailedRuns'))
})
await delay(100)
writeFileSync(join(home, 'api-results.json'), JSON.stringify(results, null, 2))
console.log(`${results.filter(result => result.result === 'pass').length}/${results.length} checks passed`)
if (results.some(result => result.result === 'fail')) process.exitCode = 1
