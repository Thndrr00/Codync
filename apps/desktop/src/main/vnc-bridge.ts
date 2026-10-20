import { randomBytes } from 'node:crypto'
import { createServer } from 'node:http'
import { connect } from 'node:net'
import { createWebSocketStream, WebSocketServer } from 'ws'

export interface VncBridge {
  /** Private capability URL, shared only with this computer's screen window. */
  url: string
  close: () => void
}

/** Authenticated loopback WebSocket to an SSH-owned, loopback-only VNC forward. */
export function bridgeVnc(vncPort: number, allowedOrigin: string | readonly string[]): Promise<VncBridge> {
  if (!Number.isInteger(vncPort) || vncPort < 1 || vncPort > 65535) {
    throw new Error('Invalid screen tunnel port.')
  }
  const origins = new Set(typeof allowedOrigin === 'string' ? [allowedOrigin] : allowedOrigin)
  if (!origins.size || origins.has('')) throw new Error('A screen renderer origin is required.')
  const path = `/screen/${randomBytes(32).toString('hex')}`
  const server = createServer((_req, res) => res.writeHead(403).end())
  const websocket = new WebSocketServer({ noServer: true, maxPayload: 1024 * 1024, perMessageDeflate: false })
  let closed = false
  const connections = new Set<() => void>()
  const close = () => {
    if (closed) return
    closed = true
    for (const disconnect of connections) disconnect()
    websocket.close()
    server.close()
  }
  return new Promise((resolve, reject) => {
    server.on('error', (error) => { close(); reject(error) })
    server.listen(0, '127.0.0.1', () => {
      const address = server.address()
      if (!address || typeof address === 'string') {
        close()
        reject(new Error("Couldn't open the screen bridge."))
        return
      }
      server.on('upgrade', (request, socket, head) => {
        socket.on('error', () => socket.destroy())
        if (closed || request.method !== 'GET' || request.url !== path ||
            request.headers.host !== `127.0.0.1:${address.port}` || !origins.has(request.headers.origin ?? '')) {
          socket.end('HTTP/1.1 403 Forbidden\r\nConnection: close\r\nContent-Length: 0\r\n\r\n')
          return
        }
        websocket.handleUpgrade(request, socket, head, (client) => {
          const stream = createWebSocketStream(client)
          const tcp = connect(vncPort, '127.0.0.1')
          const disconnect = () => {
            connections.delete(disconnect)
            tcp.destroy()
            stream.destroy()
            client.terminate()
          }
          connections.add(disconnect)
          // Stream piping applies backpressure in both directions; ws validates frames and bounds messages.
          tcp.setTimeout(10_000, disconnect)
          tcp.once('connect', () => tcp.setTimeout(0))
          tcp.on('error', disconnect)
          tcp.on('close', disconnect)
          stream.on('error', disconnect)
          stream.on('close', disconnect)
          tcp.pipe(stream).pipe(tcp)
        })
      })
      resolve({ url: `ws://127.0.0.1:${address.port}${path}`, close })
    })
  })
}
