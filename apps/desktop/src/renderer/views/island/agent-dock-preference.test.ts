import assert from 'node:assert/strict'
import { test } from 'node:test'
import {
  AGENT_DOCK_MAX_SEATS, AGENT_DOCK_SIZE, defaultAgentDockPreference,
  dockSeats, normalizeAgentDockPreference,
} from './agent-dock-preference.ts'

test('a malformed dock preference falls back field by field', () => {
  assert.deepEqual(normalizeAgentDockPreference(null), defaultAgentDockPreference)
  assert.deepEqual(normalizeAgentDockPreference([]), defaultAgentDockPreference)
  assert.deepEqual(normalizeAgentDockPreference({ edge: 'top', autoHide: 'yes', sizePercent: -5 }), {
    ...defaultAgentDockPreference, sizePercent: AGENT_DOCK_SIZE.min,
  })
  assert.deepEqual(normalizeAgentDockPreference({ shown: true, edge: 'left', sizePercent: 1000, seats: ['  a  ', 'a', 42, ''] }), {
    shown: true, edge: 'left', sizePercent: AGENT_DOCK_SIZE.max, autoHide: defaultAgentDockPreference.autoHide, seats: ['a'],
  })
})

test('seat lists are unique, capped and dropped when their bot leaves the roster', () => {
  const many = Array.from({ length: AGENT_DOCK_MAX_SEATS + 3 }, (_, i) => `bot-${i}`)
  const saved = normalizeAgentDockPreference({ seats: many })
  assert.equal(saved.seats.length, AGENT_DOCK_MAX_SEATS)
  assert.deepEqual(dockSeats(saved, many), many.slice(0, AGENT_DOCK_MAX_SEATS))
  assert.deepEqual(dockSeats(saved, ['bot-9', 'bot-0']), ['bot-0'])
  assert.deepEqual(dockSeats(defaultAgentDockPreference, many), many.slice(0, 3))
})
