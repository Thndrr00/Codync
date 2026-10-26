// Inspect the real installed Electron renderer when the native Computer Use bridge is unavailable.
import assert from 'node:assert/strict'
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { setTimeout as delay } from 'node:timers/promises'

const [port = '29480', folderArg = 'build/windows-ui', actionPath] = process.argv.slice(2)
const pages = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()
const page = pages.find(page => page.type === 'page' && page.url.includes('window=chat'))
assert.ok(page, 'Installed app chat renderer is available')
const socket = new WebSocket(page.webSocketDebuggerUrl)
await new Promise((resolve, reject) => { socket.onopen = resolve; socket.onerror = reject })
let sequence = 0
const pending = new Map()
socket.onmessage = event => {
  const message = JSON.parse(event.data)
  const request = pending.get(message.id)
  if (!request) return
  pending.delete(message.id)
  clearTimeout(request.timer)
  if (message.error) request.reject(Error(message.error.message))
  else request.resolve(message.result)
}
const call = (method, params = {}) => new Promise((resolve, reject) => {
  const id = ++sequence
  const timer = setTimeout(() => { pending.delete(id); reject(Error(`Timed out: ${method}`)) }, 15000)
  pending.set(id, { resolve, reject, timer })
  socket.send(JSON.stringify({ id, method, params }))
})
async function evaluate(expression) {
  const response = await call('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true })
  assert.ok(!response.exceptionDetails, JSON.stringify(response.exceptionDetails))
  return response.result.value
}
try {
  if (actionPath) {
    const action = JSON.parse(readFileSync(actionPath, 'utf8'))
    if (action.type === 'click') {
      await call('Page.bringToFront')
      const point = await evaluate(`(() => {
        const label = ${JSON.stringify(action.label)};
        const nodes = [...document.querySelectorAll('button, [role="button"]')].filter(node => node.getClientRects().length);
        const matches = nodes.filter(node => [node.getAttribute('aria-label'), node.title, node.textContent.trim()].includes(label));
        if (matches.length !== 1) throw Error('Expected one visible control: '+label+'; found '+matches.length);
        matches[0].scrollIntoView({block:'center'});
        const rect = matches[0].getBoundingClientRect();
        return {x:rect.left+rect.width/2,y:rect.top+rect.height/2};
      })()`)
      await call('Input.dispatchMouseEvent', {type:'mousePressed',x:point.x,y:point.y,button:'left',clickCount:1})
      await call('Input.dispatchMouseEvent', {type:'mouseReleased',x:point.x,y:point.y,button:'left',clickCount:1})
    } else if (action.type === 'input') {
      await evaluate(`(() => {
        const matches = [...document.querySelectorAll('input,textarea')].filter(node => node.getClientRects().length && [node.getAttribute('aria-label'),node.placeholder].includes(${JSON.stringify(action.label)}));
        if (matches.length !== 1) throw Error('Expected one editable field');
        const node=matches[0]; node.focus();
        const prototype = node.tagName === 'TEXTAREA' ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
        Object.getOwnPropertyDescriptor(prototype,'value').set.call(node,${JSON.stringify(action.value)});
        node.dispatchEvent(new Event('input',{bubbles:true}));
      })()`)
    } else if (action.type === 'key') {
      await call('Input.dispatchKeyEvent', { type: 'keyDown', key: action.key, code: action.code ?? action.key, modifiers: action.modifiers ?? 0 })
      await call('Input.dispatchKeyEvent', { type: 'keyUp', key: action.key, code: action.code ?? action.key, modifiers: action.modifiers ?? 0 })
    } else throw Error('Unsupported action')
    await delay(700)
  }
  const state = await evaluate(`({
    title:document.title,text:document.body.innerText.slice(0,14000),
    controls:[...document.querySelectorAll('button,[role="button"]')].filter(node=>node.getClientRects().length).map(node=>({text:node.textContent.trim(),label:node.getAttribute('aria-label'),title:node.title,disabled:node.disabled})),
    inputs:[...document.querySelectorAll('input,textarea')].filter(node=>node.getClientRects().length).map(node=>({label:node.getAttribute('aria-label'),placeholder:node.placeholder,value:node.value}))
  })`)
  const folder = resolve(folderArg)
  mkdirSync(folder, { recursive: true })
  const screenshot = await call('Page.captureScreenshot', { format: 'png' })
  writeFileSync(join(folder, 'latest.png'), Buffer.from(screenshot.data, 'base64'))
  writeFileSync(join(folder, 'latest.json'), JSON.stringify(state, null, 2))
  console.log(JSON.stringify(state, null, 2))
  console.log(`Screenshot: ${join(folder, 'latest.png')}`)
} finally { socket.close() }
