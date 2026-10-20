import assert from 'node:assert/strict'
import { createServer } from 'node:net'
import { test } from 'node:test'
import WebSocket from 'ws'
import { bridgeVnc } from './vnc-bridge.ts'

const origin = 'http://localhost:5173'

test('the bridge copies binary bytes in both directions and closes connections', async () => {
  const tcp = createServer((socket) => {
    socket.on('data', (chunk) => socket.write(Buffer.concat([Buffer.from('out:'), chunk])))
  })
  await new Promise<void>((resolve) => tcp.listen(0, '127.0.0.1', resolve))
  const address = tcp.address()
  assert.ok(address && typeof address === 'object')
  const bridge = await bridgeVnc(address.port, origin)
  const ws = new WebSocket(bridge.url, { origin })
  try {
    await new Promise<void>((resolve, reject) => { ws.once('open', resolve); ws.once('error', reject) })
    const received = new Promise<Buffer>((resolve, reject) => {
      ws.once('message', (data, binary) => binary ? resolve(Buffer.from(data as Buffer)) : reject(new Error('expected binary')))
      ws.once('error', reject)
    })
    ws.send(Buffer.from('click'))
    assert.equal((await received).toString(), 'out:click')
    const closed = new Promise<void>((resolve) => ws.once('close', () => resolve()))
    bridge.close()
    await closed
  } finally {
    ws.terminate()
    bridge.close()
    await new Promise<void>((resolve) => tcp.close(() => resolve()))
  }
})

test('the bridge rejects foreign origins and incorrect capability paths before connecting to VNC', async () => {
  let connections = 0
  const tcp = createServer((socket) => { connections++; socket.destroy() })
  await new Promise<void>((resolve) => tcp.listen(0, '127.0.0.1', resolve))
  const address = tcp.address()
  assert.ok(address && typeof address === 'object')
  const bridge = await bridgeVnc(address.port, origin)
  try {
    for (const [url, requestOrigin] of [[bridge.url, 'https://example.com'], [bridge.url + '/wrong', origin], [bridge.url, undefined]] as const) {
      await new Promise<void>((resolve, reject) => {
        const ws = new WebSocket(url, { origin: requestOrigin })
        ws.once('open', () => { ws.terminate(); reject(new Error('unauthorized connection accepted')) })
        ws.once('error', (error) => { assert.match(error.message, /403/); resolve() })
      })
    }
    await new Promise<void>((resolve, reject) => {
      const ws = new WebSocket(bridge.url, { origin, headers: { Host: 'untrusted.example' } })
      ws.once('open', () => { ws.terminate(); reject(new Error('foreign Host accepted')) })
      ws.once('error', (error) => { assert.match(error.message, /403/); resolve() })
    })
    assert.equal(connections, 0)
  } finally {
    bridge.close()
    await new Promise<void>((resolve) => tcp.close(() => resolve()))
  }
})
