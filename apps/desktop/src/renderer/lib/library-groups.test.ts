import assert from 'node:assert/strict'
import { test } from 'node:test'
import { isRecent, libraryGroups } from './library-groups.ts'

test('the library groups by the day things were published or sent', () => {
  const now = new Date(2026, 9, 24, 16, 40).getTime()
  const at = (day: number, hour = 9) => ({ at: new Date(2026, 9, day, hour).getTime() })
  const items = [at(24, 16), at(24, 0), at(23, 23), at(18), at(17, 23), at(1)]
  assert.deepEqual(
    libraryGroups(items, now).map((g) => [g.group, g.items.length]),
    [['Today', 2], ['Yesterday', 1], ['This week', 1], ['Earlier', 2]],
  )
  assert.deepEqual(libraryGroups([], now), [])
})

test('Recent is the last seven days, not the calendar week', () => {
  const now = new Date(2026, 9, 24, 16, 40).getTime()
  const day = 24 * 60 * 60 * 1000
  assert.equal(isRecent(now - 6 * day, now), true)
  assert.equal(isRecent(now - 7 * day, now), false)
  assert.equal(isRecent(now + day, now), false, 'a future timestamp is not recent')
})
