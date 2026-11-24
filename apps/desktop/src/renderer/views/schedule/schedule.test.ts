import assert from 'node:assert/strict'
import { test } from 'node:test'
import { scheduleRows, type ScheduleItem } from './schedule.ts'

const item = (id: string, at: number, paused?: boolean): ScheduleItem => ({
  id, name: id, at, timezone: 'UTC', trigger: 'cron', botId: 'bot', kind: 'routine', paused,
})

test('enabled routines keep every occurrence', () => {
  const rows = scheduleRows([item('a', 1000), item('a', 2000), item('b', 3000)], 'UTC')
  assert.deepEqual(rows.map((row) => [row.key, row.paused]), [
    ['a:1000', false], ['a:2000', false], ['b:3000', false],
  ])
})

test('a paused routine keeps one row per day, in the chosen zone', () => {
  const first = Date.parse('2026-09-28T01:00:00Z')
  const sameDay = Date.parse('2026-09-28T10:00:00Z')
  const nextDay = Date.parse('2026-09-29T01:00:00Z')
  const live = Date.parse('2026-09-28T18:00:00Z')
  const rows = scheduleRows([item('paused', first, true), item('paused', sameDay, true), item('paused', nextDay, true), item('live', live)], 'Asia/Taipei')
  assert.deepEqual(rows.map((row) => ({ id: row.item.id, at: row.item.at, paused: row.paused })), [
    { id: 'paused', at: first, paused: true },
    { id: 'live', at: live, paused: false },
    { id: 'paused', at: nextDay, paused: true },
  ])
  assert.equal(new Set(rows.filter((row) => row.paused).map((row) => row.key)).size, 2)
})

test('rows stay ordered by instant even when the host sends them out of order', () => {
  const rows = scheduleRows([item('late', 3000), item('early', 1000, true), item('mid', 2000)], 'UTC')
  assert.deepEqual(rows.map((row) => row.item.at), [1000, 2000, 3000])
})
