import assert from 'node:assert/strict'
import { test } from 'node:test'
import { NO_ROSTER_VIEW_FILTER } from './roster-view-filter.ts'
import { rosterTab, selectRosterTab } from './roster-tabs.ts'

const counts = { all: 7, unread: 3, folders: { Work: 2, 'Work/Clients': 1, Personal: 0 } }

test('the roster tabs show All, Unread and folders with their unread badges', () => {
  const view = rosterTab(counts)
  assert.deepEqual(view.tabs.map((tab) => [tab.id, tab.label, tab.badge]), [
    ['all', 'All', 7],
    ['unread', 'Unread', 3],
    ['Personal', 'Personal', 0],
    ['Work', 'Work', 2],
    ['Work/Clients', 'Clients', 1],
  ])
  assert.equal(view.selected, 'all')
})

test('a negative or missing tally never becomes a negative badge', () => {
  const view = rosterTab({ all: -2, unread: 0, folders: { Work: -1 } })
  assert.deepEqual(view.tabs.map((tab) => tab.badge), [0, 0, 0])
})

test('the selected tab follows the view filter, and one outside the tabs falls back to All', () => {
  assert.equal(rosterTab(counts, { ...NO_ROSTER_VIEW_FILTER, unread: true }).selected, 'unread')
  assert.equal(rosterTab(counts, { ...NO_ROSTER_VIEW_FILTER, sections: ['Work/Clients'] }).selected, 'Work/Clients')
  assert.equal(rosterTab(counts, { ...NO_ROSTER_VIEW_FILTER, sections: ['Gone'] }).selected, 'all')
  assert.equal(rosterTab(counts, { ...NO_ROSTER_VIEW_FILTER, sections: ['Work', 'Work/Clients'] }).selected, 'all')
})

test('a tab click changes only unread and folders, keeping the kind, states and sort', () => {
  const current = { ...NO_ROSTER_VIEW_FILTER, show: 'channels' as const, sort: 'recent' as const, working: true, sections: ['Personal'] }
  assert.deepEqual(selectRosterTab(current, 'unread'), { ...current, unread: true, sections: [] })
  assert.deepEqual(selectRosterTab(current, 'Work'), { ...current, unread: false, sections: ['Work'] })
  assert.deepEqual(selectRosterTab({ ...current, unread: true }, 'all'), { ...current, unread: false, sections: [] })
})
