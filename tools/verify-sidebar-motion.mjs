// Compare production sidebar CSS in installed Chromium, without building Codync.
import assert from 'node:assert/strict'
import { existsSync, readFileSync, mkdtempSync, rmSync } from 'node:fs'
import { join, dirname, resolve } from 'node:path'
import { tmpdir } from 'node:os'
import { spawn, execFileSync } from 'node:child_process'
import { createInterface } from 'node:readline'
import { setTimeout as delay } from 'node:timers/promises'

const relative = 'apps/desktop/src/renderer/views/chat-window.css'
const before = execFileSync('git', ['show', `HEAD:${relative}`], { encoding: 'utf8' })
const after = readFileSync(relative, 'utf8')
const theme = readFileSync('apps/desktop/src/renderer/styles/theme.css', 'utf8')
const paths = ['PROGRAMFILES', 'PROGRAMFILES(X86)', 'LOCALAPPDATA'].flatMap(key => process.env[key]
  ? ['Google/Chrome/Application/chrome.exe', 'Microsoft/Edge/Application/msedge.exe'].map(path => join(process.env[key], path)) : [])
const executable = paths.find(existsSync)
assert.ok(process.platform === 'win32' && executable, 'Installed Windows Chrome/Edge required')
const parent = resolve(tmpdir())
const folder = mkdtempSync(join(parent, 'codync-sidebar-probe-'))
const broker = readFileSync('host/src/screen/protected-browser-broker.mjs', 'utf8')
const child = spawn(process.execPath, ['-e', broker, '--', executable, folder, '--headless-probe'], {
  stdio: ['pipe', 'pipe', 'ignore'], windowsHide: true,
})
let sequence = 0
const pending = new Map()
const lines = createInterface({ input: child.stdout })
lines.on('line', line => {
  const reply = JSON.parse(line)
  const request = pending.get(reply.id)
  if (!request) return
  pending.delete(reply.id)
  clearTimeout(request.timer)
  if (reply.error || reply.result?.exceptionDetails) request.reject(Error('Browser refused fixture'))
  else request.resolve(reply.result)
})
const call = (method, params = {}, sessionId) => new Promise((resolve, reject) => {
  const id = ++sequence
  const timer = setTimeout(() => { pending.delete(id); reject(Error(`Timeout: ${method}`)) }, 10000)
  pending.set(id, { resolve, reject, timer })
  child.stdin.write(JSON.stringify({ id, method, params, ...(sessionId ? { sessionId } : {}) }) + '\n')
})
try {
  const { targetId } = await call('Target.createTarget', { url: 'about:blank' })
  const { sessionId } = await call('Target.attachToTarget', { targetId, flatten: true })
  await call('Target.activateTarget', { targetId })
  await call('Page.bringToFront', {}, sessionId)
  await call('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: 'no-preference' }] }, sessionId)
  const evaluate = async expression => (await call('Runtime.evaluate', { expression, returnByValue: true }, sessionId)).result.value
  const fixture = `<div class="split"><aside class="sidebar" style="width:300px"><div class="sidebar-header">Controls</div><div class="roster">${'<div class="roster-row">Agent with a long title and activity text</div>'.repeat(150)}</div></aside><main class="detail">Conversation</main></div>`
  await call('Performance.enable', {}, sessionId)
  const layouts = async () => (await call('Performance.getMetrics', {}, sessionId)).metrics.find(m => m.name === 'LayoutCount').value
  const measure = async css => {
    await evaluate(`document.documentElement.dataset.platform='win32'; document.head.innerHTML='';
      document.head.appendChild(Object.assign(document.createElement('style'),{textContent:${JSON.stringify(theme)}}));
      document.head.appendChild(Object.assign(document.createElement('style'),{textContent:${JSON.stringify(css)}}));
      document.body.innerHTML=${JSON.stringify(fixture)}; document.body.style.height='700px'; document.body.offsetWidth`)
    await delay(400)
    const start = await layouts()
    await evaluate(`document.querySelector('.sidebar').classList.add('compact'); document.querySelector('.sidebar').style.width='76px'; document.body.offsetWidth`)
    await delay(400)
    return await layouts() - start
  }
  const oldLayouts = await measure(before)
  const newLayouts = await measure(after)
  if (before !== after) assert.ok(newLayouts < oldLayouts, `Expected fewer layouts: ${oldLayouts} -> ${newLayouts}`)
  assert.equal(await evaluate(`getComputedStyle(document.querySelector('.sidebar')).transitionProperty`), 'none')
  assert.equal(await evaluate(`getComputedStyle(document.querySelector('.sidebar-header')).transitionProperty`), 'opacity, transform')
  await evaluate(`document.querySelector('.sidebar').classList.add('resizing')`)
  assert.equal(await evaluate(`getComputedStyle(document.querySelector('.roster')).animationName`), 'none')
  await evaluate(`document.querySelector('.sidebar').classList.remove('resizing')`)
  assert.equal(await evaluate(`getComputedStyle(document.querySelector('.roster')).animationName`), 'none')
  await call('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: 'reduce' }] }, sessionId)
  await evaluate(`document.querySelector('.sidebar').classList.remove('resizing')`)
  assert.equal(await evaluate(`getComputedStyle(document.querySelector('.roster')).animationName`), 'none')
  assert.equal(await evaluate(`getComputedStyle(document.querySelector('.sidebar-header')).transitionDuration`), '0s, 0s')
  await evaluate(`document.documentElement.dataset.platform='linux'`)
  assert.equal(await evaluate(`getComputedStyle(document.querySelector('.sidebar')).transitionProperty`), 'width')
  console.log(JSON.stringify({ status: 'passed', collapseLayoutCount: { before: oldLayouts, after: newLayouts },
    checks: ['production CSS', 'no width/padding transition on Windows', 'direct resize and release', 'reduced motion', 'other-platform CSS unchanged'],
    scope: 'Chromium CSS fixture, not full native-app e2e' }))
  await call('Browser.close')
} finally {
  lines.close()
  for (const request of pending.values()) { clearTimeout(request.timer); request.reject(Error('Probe closed')) }
  child.stdin.end()
  child.kill()
  await delay(500)
  assert.equal(dirname(resolve(folder)), parent)
  assert.ok(folder.startsWith(join(parent, 'codync-sidebar-probe-')))
  try { rmSync(folder, { recursive: true, force: true, maxRetries: 3, retryDelay: 250 }) }
  catch { console.warn(`Windows retained temporary fixture profile: ${folder}`) }
}
