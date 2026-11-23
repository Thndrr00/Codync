import assert from 'node:assert/strict'
import { test } from 'node:test'
import {
  actionsFor,
  countTasks,
  inFilter,
  statusFilterValue,
  statusLabel,
  statusOf,
  type AgentTask,
} from './task-view-model.ts'

const task = (patch: Partial<AgentTask> = {}): AgentTask => ({
  id: 'task:1',
  botId: 'b',
  name: 'Review',
  state: 'open',
  root: 'task:r1',
  ...patch,
})

test('a run hidden behind a lifecycle change reports the lifecycle, not a stale schedule', () => {
  assert.equal(statusOf(task({ run: 'queued' })), 'queued')
  assert.equal(statusOf(task({ run: 'running' })), 'running')
  assert.equal(statusOf(task({ run: 'stopped' })), 'stopped')
  assert.equal(statusOf(task({ run: 'running', state: 'closed' })), 'paused')
  assert.equal(statusOf(task({ run: 'running', state: 'archived' })), 'archived')
  assert.equal(statusOf(task()), 'waiting')
  assert.equal(statusLabel('queued'), 'Waiting for a slot')
})

test('a stopped task can be followed up but never restarted on its own', () => {
  const stopped = actionsFor(task({ run: 'stopped' }))
  assert.equal(stopped.followUp, true)
  assert.equal(stopped.stop, false)
  assert.equal(stopped.reopen, false)
  const running = actionsFor(task({ run: 'running' }))
  assert.equal(running.stop, true)
  assert.equal(running.archive, true)
  assert.deepEqual(actionsFor(task({ state: 'archived' })), {
    open: false,
    followUp: false,
    stop: false,
    archive: false,
    reopen: true,
  })
})

test('counts cover only work still in the chat and the status filter is exact', () => {
  const tasks = [
    task({ id: '1', run: 'running', unread: 2 }),
    task({ id: '2', run: 'queued', unread: 1 }),
    task({ id: '3', run: 'stopped' }),
    task({ id: '4', state: 'archived' }),
  ]
  assert.deepEqual(countTasks(tasks), { active: 1, queued: 1, unread: 3 })
  assert.equal(statusFilterValue('all'), undefined)
  assert.equal(statusFilterValue('closed'), 'closed')
  assert.deepEqual(tasks.filter((t) => inFilter(t, 'open')).map((t) => t.id), ['1', '2', '3'])
  assert.deepEqual(tasks.filter((t) => inFilter(t, 'archived')).map((t) => t.id), ['4'])
})
