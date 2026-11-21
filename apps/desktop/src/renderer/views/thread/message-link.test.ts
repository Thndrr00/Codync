import assert from 'node:assert/strict'
import { test } from 'node:test'
import { messageLink } from './message-link.ts'

test('a message link names its bot and message in the host format', () => {
  assert.equal(messageLink('pacer', 'e-42'), 'bot:pacer/entry:e-42')
  assert.equal(messageLink('group:team', '0123456789abcdef'), 'bot:group:team/entry:0123456789abcdef')
})
