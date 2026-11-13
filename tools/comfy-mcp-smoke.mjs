#!/usr/bin/env node
// Smoke test for the official Comfy MCP (Comfy-Org/comfy-mcp), not for Codync's own bridge.
//
// It starts the official stdio server, performs the MCP handshake, lists its tools and checks the
// seven operations Codync's bridge implements. With --call it also asks for server_info and
// search_templates, which need a running ComfyUI. Nothing here ships in the app: this is the
// harness docs/reference/comfy-official-mcp.md describes for a later adoption.
//
//   node tools/comfy-mcp-smoke.mjs --bin comfy-mcp --comfy-bin /path/to/comfy --call
import { spawn } from 'node:child_process'

const argv = process.argv.slice(2)
const option = (name, fallback = null) => {
  const at = argv.indexOf(name)
  return at >= 0 && argv[at + 1] ? argv[at + 1] : fallback
}
const bin = option('--bin', process.env.COMFY_MCP_BIN || (process.platform === 'win32' ? 'comfy-mcp.exe' : 'comfy-mcp'))
const comfyBin = option('--comfy-bin', process.env.COMFY_BIN)
const call = argv.includes('--call')

const required = [
  ['list templates', 'search_templates'],
  ['describe template', 'get_template'],
  ['run by name', 'run_template'],
  ['run a graph', 'run_workflow'],
  ['job status', 'job'],
  ['fetch outputs', 'fetch_outputs'],
  ['upload input', 'upload_file'],
]

const child = spawn(bin, [], {
  env: { ...process.env, ...(comfyBin ? { COMFY_BIN: comfyBin } : {}) },
  stdio: ['pipe', 'pipe', 'inherit'],
})

let nextId = 1
let buffer = ''
const pending = new Map()

child.stdout.setEncoding('utf8')
child.stdout.on('data', (chunk) => {
  buffer += chunk
  let newline
  while ((newline = buffer.indexOf('\n')) >= 0) {
    const line = buffer.slice(0, newline).trim()
    buffer = buffer.slice(newline + 1)
    if (!line) continue
    try {
      const message = JSON.parse(line)
      const settle = pending.get(message.id)
      if (!settle) continue
      pending.delete(message.id)
      message.error ? settle.reject(new Error(message.error.message ?? 'MCP error')) : settle.resolve(message.result)
    } catch {
      // A log line from the server, not a JSON-RPC message.
    }
  }
})

function request(method, params = {}, timeoutMs = 15_000) {
  const id = nextId++
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      pending.delete(id)
      reject(new Error(`${method} timed out`))
    }, timeoutMs)
    pending.set(id, {
      resolve: (value) => { clearTimeout(timer); resolve(value) },
      reject: (error) => { clearTimeout(timer); reject(error) },
    })
    child.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', id, method, params })}\n`)
  })
}

function notify(method, params = {}) {
  child.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', method, params })}\n`)
}

child.on('error', (error) => {
  console.error(`Could not start ${bin}: ${error.message}`)
  process.exit(2)
})

try {
  await request('initialize', {
    protocolVersion: '2024-11-05',
    capabilities: {},
    clientInfo: { name: 'codync-comfy-mcp-smoke', version: '1.0.0' },
  })
  notify('notifications/initialized')

  const { tools } = await request('tools/list')
  const names = new Set(tools.map((tool) => tool.name))
  console.log(`Official Comfy MCP: ${tools.length} tools from ${bin}`)
  console.log('')
  console.log('operation          tool                status')
  console.log('-----------------  ------------------  ------')
  let missing = 0
  for (const [operation, tool] of required) {
    const ok = names.has(tool)
    if (!ok) missing++
    console.log(`${operation.padEnd(17)}  ${tool.padEnd(18)}  ${ok ? 'ok' : 'MISSING'}`)
  }

  if (call) {
    const info = await request('tools/call', { name: 'server_info', arguments: {} })
    console.log('\nserver_info:\n' + JSON.stringify(info, null, 2))
    const templates = await request('tools/call', { name: 'search_templates', arguments: { limit: 5 } })
    console.log('\nsearch_templates (5):\n' + JSON.stringify(templates, null, 2))
  }

  child.kill()
  process.exit(missing === 0 ? 0 : 1)
} catch (error) {
  child.kill()
  console.error(error.message)
  process.exit(2)
}
