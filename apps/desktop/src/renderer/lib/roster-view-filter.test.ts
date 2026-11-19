import assert from 'node:assert/strict'
import { test } from 'node:test'
import { normalizeBot } from '../../shared/models.ts'
import {
  NO_ROSTER_VIEW_FILTER, matchesRosterView, normalizeRosterViewFilter, rosterSectionMark, rosterSectionTree,
  rosterViewFilterIsSet, sortRosterRows, toggleRosterSection,
} from './roster-view-filter.ts'

const agent = normalizeBot({ id: 'a', name: 'Nova', section: 'Work/Clients', unread: 2 })
const channel = normalizeBot({ id: 'g', kind: 'group', name: 'Ship room', section: 'Work' })
const idle = normalizeBot({ id: 'b', name: 'Kai' })
const working = normalizeBot({ id: 'c', name: 'Ada', status: 'working', section: 'Personal' })

test('The roster view filter keeps the open bot visible when its own row no longer matches', () => {
  const agentsOnly = { ...NO_ROSTER_VIEW_FILTER, show: 'agents' as const }
  assert.equal(matchesRosterView(agent, agentsOnly), true)
  assert.equal(matchesRosterView(channel, agentsOnly), false)
  const workingOnly = { ...NO_ROSTER_VIEW_FILTER, working: true }
  assert.equal(matchesRosterView(working, workingOnly), true)
  assert.equal(matchesRosterView(idle, workingOnly), false)
})

test('State toggles combine: a row shows when it has either state', () => {
  const either = { ...NO_ROSTER_VIEW_FILTER, unread: true, working: true }
  assert.equal(matchesRosterView(agent, either), true)
  assert.equal(matchesRosterView(working, either), true)
  assert.equal(matchesRosterView(idle, either), false)
})

test('A section filter includes the sections nested in it, and each one can come back out', () => {
  const work = { ...NO_ROSTER_VIEW_FILTER, sections: ['Work', 'Work/Clients'] }
  assert.equal(matchesRosterView(agent, work), true)
  assert.equal(matchesRosterView(channel, work), true)
  assert.equal(matchesRosterView(working, work), false)
  const parentOnly = { ...NO_ROSTER_VIEW_FILTER, sections: ['Work'] }
  assert.equal(matchesRosterView(channel, parentOnly), true)
  assert.equal(matchesRosterView(agent, parentOnly), false)
  const clients = { ...NO_ROSTER_VIEW_FILTER, sections: ['Work/Clients'] }
  assert.equal(matchesRosterView(agent, clients), true)
  assert.equal(matchesRosterView(channel, clients), false)
})

test('The section tree nests names and keeps the rows without a section last', () => {
  const roots = rosterSectionTree([agent, channel, idle, working])
  assert.deepEqual(roots.map((node) => node.name), ['Personal', 'Work', ''])
  assert.deepEqual(roots[1]!.children.map((node) => node.name), ['Work/Clients'])
  assert.equal(roots[2]!.label, 'Bots')
})

test('Selecting a section marks it and its folders, and all selected reads as none', () => {
  const roots = rosterSectionTree([agent, channel, idle, working])
  const work = roots[1]!
  assert.equal(rosterSectionMark(work, ['Work/Clients']), 'mixed')
  assert.deepEqual(toggleRosterSection(work, roots, []), ['Work', 'Work/Clients'])
  assert.deepEqual(toggleRosterSection(work, roots, ['Work']), ['Work', 'Work/Clients'])
  assert.deepEqual(toggleRosterSection(work, roots, ['Personal', 'Work', 'Work/Clients']), ['Personal'])
  assert.deepEqual(toggleRosterSection(work, roots, ['Work', 'Work/Clients']), [])
})

test('A saved filter of another shape reads as far as it can', () => {
  assert.deepEqual(normalizeRosterViewFilter('agents'), NO_ROSTER_VIEW_FILTER)
  assert.deepEqual(normalizeRosterViewFilter({ show: 'channels', unread: true, sections: ['Work', 2, 'Work'] }), {
    ...NO_ROSTER_VIEW_FILTER, show: 'channels', unread: true, sections: ['Work'],
  })
  assert.equal(normalizeRosterViewFilter({ sort: 'recent' }).sort, 'recent')
  assert.equal(normalizeRosterViewFilter({ sort: 'other' }).sort, 'manual')
  assert.equal(rosterViewFilterIsSet({ ...NO_ROSTER_VIEW_FILTER, sections: ['Work'] }), true)
  assert.equal(rosterViewFilterIsSet(NO_ROSTER_VIEW_FILTER), false)
  // The sort is an order, not a filter: it does not light the filter's dot.
  assert.equal(rosterViewFilterIsSet({ ...NO_ROSTER_VIEW_FILTER, sort: 'recent' }), false)
})

test('Recent activity orders a section by last message, keeping pins and needs-attention on top', () => {
  const old = normalizeBot({ id: 'old', name: 'Old', section: 'Work', lastAt: 100 })
  const newest = normalizeBot({ id: 'newest', name: 'New', section: 'Work', lastAt: 300 })
  const middle = normalizeBot({ id: 'middle', name: 'Mid', section: 'Work', lastAt: 200 })
  const waiting = normalizeBot({ id: 'waiting', name: 'Ask', section: 'Work', status: 'needsInput', lastAt: 50 })
  const failed = normalizeBot({ id: 'failed', name: 'Error', section: 'Work', status: 'error', lastAt: 40 })
  const pinned = normalizeBot({ id: 'pinned', name: 'Pin', section: 'Work', pinned: true, lastAt: 10 })
  // The saved drag order: neither sort changes it, so switching back restores it.
  const rows = [old, newest, middle, waiting, failed, pinned].map((bot) => ({ bot }))
  const ids = (sorted: typeof rows) => sorted.map((row) => row.bot.id)

  assert.deepEqual(ids(sortRosterRows(rows, 'manual')), ['old', 'newest', 'middle', 'waiting', 'failed', 'pinned'])
  assert.deepEqual(ids(sortRosterRows(rows, 'recent')), ['pinned', 'waiting', 'failed', 'newest', 'middle', 'old'])
  assert.deepEqual(ids(rows), ['old', 'newest', 'middle', 'waiting', 'failed', 'pinned'])
  assert.deepEqual(ids(sortRosterRows(rows, 'manual')), ['old', 'newest', 'middle', 'waiting', 'failed', 'pinned'])
})
