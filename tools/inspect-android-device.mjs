// Non-destructive, opt-in UI checks on a connected Codync phone. Never clears app data.
import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { mkdirSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { setTimeout as delay } from 'node:timers/promises'

const [serial, folderArg, action, value] = process.argv.slice(2)
assert.ok(serial && folderArg, 'Usage: node tools/inspect-android-device.mjs <serial> <folder> [tap <visible label>|text <ASCII>|back]')
const folder = resolve(folderArg)
const adb = (...args) => execFileSync('adb', ['-s', serial, ...args], { timeout: 30000, windowsHide: true })
const decode = value => value.replace(/&quot;/g, '"').replace(/&apos;/g, "'").replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&amp;/g, '&')
function nodes(xml) {
  return [...xml.matchAll(/<node\s+([^>]+)>/g)].map(match =>
    Object.fromEntries([...match[1].matchAll(/([\w-]+)="([^"]*)"/g)].map(([, key, val]) => [key, decode(val)])),
  ).filter(node => (node.text || node['content-desc']) && node.bounds !== '[0,0][0,0]')
}
async function inspect() {
  let focus = ''
  for (let attempt = 0; attempt < 3; attempt++) {
    focus = adb('shell', 'dumpsys', 'window').toString().split('\n').find(line => line.includes('mCurrentFocus=')) ?? ''
    if (focus && !focus.includes('mCurrentFocus=null')) break
    await delay(500)
  }
  assert.match(focus, /com\.pokai\.Codync\.android\.dev/, 'Unlock the phone and show Codync Dev first')
  adb('shell', 'uiautomator', 'dump', '/sdcard/codync-validation.xml')
  const xml = adb('shell', 'cat', '/sdcard/codync-validation.xml').toString()
  return { xml, controls: nodes(xml) }
}
let state = await inspect()
if (action === 'tap') {
  const candidates = state.controls.filter(node => node.text === value || node['content-desc'] === value)
  const clickable = candidates.filter(node => node.clickable === 'true')
  const matches = clickable.length ? clickable : candidates
  assert.equal(matches.length, 1, `Expected one visible control matching ${value}`)
  const bounds = matches[0].bounds.match(/\d+/g).map(Number)
  assert.equal(bounds.length, 4)
  adb('shell', 'input', 'tap', String(Math.floor((bounds[0] + bounds[2]) / 2)), String(Math.floor((bounds[1] + bounds[3]) / 2)))
  await delay(700)
  state = await inspect()
} else if (action === 'text') {
  assert.match(value, /^[A-Za-z0-9 .,:?!_-]+$/, 'Only plain ASCII test text is supported')
  adb('shell', 'input', 'text', value.replaceAll(' ', '%s'))
  await delay(400)
  state = await inspect()
} else if (action === 'back') {
  adb('shell', 'input', 'keyevent', '4')
  await delay(700)
  state = await inspect()
} else assert.ok(!action, 'Unsupported action')
mkdirSync(folder, { recursive: true })
writeFileSync(join(folder, 'screen.xml'), state.xml)
writeFileSync(join(folder, 'screen.png'), adb('exec-out', 'screencap', '-p'))
writeFileSync(join(folder, 'controls.json'), JSON.stringify(state.controls, null, 2))
console.log(JSON.stringify(state.controls.map(node => ({ text: node.text, label: node['content-desc'], bounds: node.bounds, enabled: node.enabled })), null, 2))
