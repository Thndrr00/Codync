import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { BotStatus } from './models.ts'
import type { IslandBot } from './island.ts'
import { agentDockCounts, agentDockLine, agentDockRows, agentDockState, agentDockText } from './island-agents.ts'

const bot = (ref: string, status: BotStatus | string, over: Partial<IslandBot> = {}): IslandBot => ({
  ref, name: ref, shape: 'blob', color: 'blue', status, backend: 'Claude Code', folder: 'site', detail: `${ref} works`, steps: [], error: null, ...over,
})

test('the dock keeps the roster order the chat window sends', () => {
  const rows = agentDockRows([bot('a', 'idle'), bot('b', 'working'), bot('c', 'needsInput'), bot('d', 'error')])
  assert.deepEqual(rows.map((row) => row.ref), ['a', 'b', 'c', 'd'])
  assert.deepEqual(rows.map((row) => row.state), ['idle', 'working', 'needsInput', 'error'])
})

test('a status this app is newer than counts as idle, and unread stays a whole count', () => {
  assert.equal(agentDockState('paused'), 'idle')
  assert.equal(agentDockState('working'), 'working')
  const [row] = agentDockRows([bot('a', 'idle', { unread: 4.7 })])
  assert.equal(row?.unread, 4)
  assert.equal(agentDockRows([bot('a', 'idle', { unread: -2 })])[0]?.unread, 0)
  assert.equal(agentDockRows([bot('a', 'idle')])[0]?.unread, 0)
})

test('a row says why a bot stopped before what it was doing', () => {
  const rows = agentDockRows([bot('a', 'error', { error: 'The agent signed out', detail: 'Editing a file' }), bot('b', 'working', { detail: 'Running the tests' })])
  assert.equal(rows[0]?.activity, 'The agent signed out')
  assert.equal(rows[1]?.activity, 'Running the tests')
})

test('the line needs you first, then counts working, stopped and idle agents', () => {
  const rows = (...states: Parameters<typeof bot>[1][]) => agentDockRows(states.map((status, i) => bot(String(i), status)))
  assert.deepEqual(agentDockLine(rows('needsInput', 'working', 'working')), { key: 'Needs you', count: null })
  assert.deepEqual(agentDockLine(rows('needsInput', 'needsInput')), { key: '{count} need you', count: 2 })
  assert.deepEqual(agentDockLine(rows('working')), { key: '1 agent working', count: null })
  assert.deepEqual(agentDockLine(rows('working', 'working', 'idle')), { key: '{count} agents working', count: 2 })
  assert.deepEqual(agentDockLine(rows('error')), { key: '1 agent stopped', count: null })
  assert.deepEqual(agentDockLine(rows('error', 'error')), { key: '{count} agents stopped', count: 2 })
  assert.deepEqual(agentDockLine(rows('idle', 'idle')), { key: 'All agents idle', count: null })
  assert.deepEqual(agentDockLine([]), { key: 'No agents on this computer', count: null })
})

test('the counts add up, and the line fills its count after translation', () => {
  const rows = agentDockRows([bot('a', 'working'), bot('b', 'needsInput'), bot('c', 'error'), bot('d', 'idle'), bot('e', 'idle')])
  assert.deepEqual(agentDockCounts(rows), { working: 1, needsYou: 1, failed: 1, idle: 2 })
  assert.equal(agentDockText(agentDockLine(rows)), 'Needs you')
  assert.equal(agentDockText({ key: '{count} agents working', count: 3 }, () => '{count} agentes trabajando'), '3 agentes trabajando')
})
