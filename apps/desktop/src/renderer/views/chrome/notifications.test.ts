import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { Bot, Entry } from '@shared/models'
import { collectNotices, questionHeadline } from './notifications.ts'

const bot = (id: string, status: Bot['status'] = 'idle', startedAt = 10): Bot => ({
  id,
  name: `Bot ${id}`,
  status,
  startedAt,
} as Bot)

const entry = (id: string, data: Entry['data'], kind = 'permission', at = 1): Entry => ({
  id,
  seq: 0,
  botId: 'b1',
  rev: 1,
  kind,
  turn: 1,
  data,
  createdAt: at,
  updatedAt: at,
})

test('open permission cards and member questions become notification items with a bot ref', () => {
  const notices = collectNotices({
    roster: [{
      ref: { computerId: 'c1', botId: 'b1' },
      bot: bot('b1'),
      entries: [
        entry('p1', { status: 'pending', toolKind: 'question' }),
        entry('p2', { status: 'answered', toolKind: 'execute' }),
        entry('q1', { memberQuestion: { status: 'pending', answers: [] } }, 'notice', 5),
      ],
    }],
    approvals: [],
  })
  assert.deepEqual(notices.map((n) => n.key), [
    'question:c1/b1:q1',
    'question:c1/b1:p1',
  ])
  assert.equal(notices[0]!.title, 'Asks you a question')
  assert.deepEqual(notices[0]!.ref, { computerId: 'c1', botId: 'b1' })
})

test('failed and needs-input bots appear once each, while working bots do not', () => {
  const notices = collectNotices({
    roster: [
      { ref: { computerId: 'c1', botId: 'b1' }, bot: bot('b1', 'needsInput'), entries: [] },
      { ref: { computerId: 'c1', botId: 'b2' }, bot: bot('b2', 'error', 20), entries: [] },
      { ref: { computerId: 'c1', botId: 'b3' }, bot: bot('b3', 'working'), entries: [] },
    ],
    approvals: [],
  })
  assert.deepEqual(notices.map((n) => n.kind), ['error', 'needsInput'])
  assert.equal(notices[0]!.detail, 'Bot b2')
})

test('device access approvals have no chat ref and can be brought back from the list', () => {
  const notices = collectNotices({
    roster: [],
    approvals: [{ id: 'c1/device', deviceName: 'Edwin’s phone', createdAt: 5 }],
  })
  assert.equal(notices.length, 1)
  assert.equal(notices[0]!.kind, 'approval')
  assert.equal(notices[0]!.detail, 'Edwin’s phone')
  assert.equal(notices[0]!.ref, null)
})

test('questions headline by the tool that asked, and empty stores give no notices', () => {
  assert.equal(questionHeadline('plan'), 'Wants your approval of a plan')
  assert.equal(questionHeadline(), 'Wants your approval')
  assert.deepEqual(collectNotices({ roster: [], approvals: [] }), [])
})
