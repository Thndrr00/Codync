import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { IslandAction } from '../../shared/island-activity.ts'
import { navigateCommand } from '../views/chat-navigation.ts'

function navigation() {
  const calls: unknown[] = []
  return {
    calls,
    handlers: {
      newBot: () => { calls.push('newBot') }, search: () => { calls.push('search') },
      network: () => { calls.push('network') }, sidebar: () => { calls.push('sidebar') },
      marketplace: (command: unknown) => { calls.push(command) },
      voice: (key: unknown) => { calls.push(['voice', key]) },
      island: (action: IslandAction) => { calls.push(action) },
    },
  }
}

test('island navigation forwards only existing flow, channel and diagnostic destinations', () => {
  const { calls, handlers } = navigation()
  const actions: IslandAction[] = [{ kind: 'openFlow', ref: 'computer/bot' }, { kind: 'openChannels' }, { kind: 'openDiagnostics' }]
  for (const action of actions) navigateCommand({ kind: 'islandAction', action }, handlers)
  assert.deepEqual(calls, actions)
})

test('host and voice mutations are not repeated by the window navigation router', () => {
  const { calls, handlers } = navigation()
  const actions: IslandAction[] = [
    { kind: 'stopBot', ref: 'computer/bot' }, { kind: 'callMute', callId: 'call-1' }, { kind: 'callEnd', callId: 'call-1' },
    { kind: 'refreshUsage' }, { kind: 'refreshActivity' },
  ]
  for (const action of actions) navigateCommand({ kind: 'islandAction', action }, handlers)
  navigateCommand({ kind: 'stopBot', ref: 'computer/bot' }, handlers)
  navigateCommand({ kind: 'sendMessage', ref: 'computer/bot', text: 'hello' }, handlers)
  assert.deepEqual(calls, [])
})

test('legacy window commands keep their existing routes and voice keys dispatch once', () => {
  const { calls, handlers } = navigation()
  navigateCommand({ kind: 'newBot' }, handlers)
  navigateCommand({ kind: 'search' }, handlers)
  navigateCommand({ kind: 'networkSettings' }, handlers)
  navigateCommand({ kind: 'toggleSidebar' }, handlers)
  navigateCommand({ kind: 'openMarketplace', section: 'apps', id: 'slack' }, handlers)
  navigateCommand({ kind: 'voiceKey', key: 'double' }, handlers)
  assert.deepEqual(calls, ['newBot', 'search', 'network', 'sidebar', { kind: 'openMarketplace', section: 'apps', id: 'slack' }, ['voice', 'double']])
})
