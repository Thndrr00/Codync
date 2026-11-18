import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { Entry, Subagent } from '../../../shared/models.ts'
import { activeSubagentCard, runningSubagents, subagentDuration } from './subagent-card.ts'

const card = (id: string, states: string[], updatedAt = 1): Entry => ({
  id,
  seq: 1,
  botId: 'bot',
  rev: 1,
  kind: 'subagents',
  turn: 1,
  createdAt: 1,
  updatedAt,
  data: {
    subagents: states.map((state, i): Subagent => ({ id: `${id}-${i}`, task: 'Read', state, step: '', startedAt: 0 })),
  },
})

test('the dock follows the newest turn whose subagents still work', () => {
  const entries = [card('a', ['running']), card('b', ['done']), card('c', ['running', 'done'])]
  assert.equal(activeSubagentCard(entries)?.id, 'c')
  assert.equal(runningSubagents(card('d', ['done', 'failed', 'stopped'])), false)
  assert.equal(activeSubagentCard([card('e', ['done'])]), undefined)
})

test('a subagent time reads in seconds, then minutes', () => {
  assert.equal(subagentDuration(12_400), '12s')
  assert.equal(subagentDuration(65_000), '1m 05s')
  assert.equal(subagentDuration(-5), '0s')
})
