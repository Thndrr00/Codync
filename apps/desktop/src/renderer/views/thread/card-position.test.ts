import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { Entry, EntryData } from '../../../shared/models.ts'
import { card_position } from './card-position.ts'

const entry = (id: string, data: EntryData = { text: 'hi' }): Entry => ({
  id, seq: 1, botId: 'bot', rev: 1, kind: 'permission', turn: 1,
  createdAt: 1, updatedAt: 1, data,
})

test('an answered card stays at the row position of its own entry', () => {
  const entries = [entry('first-q'), entry('later-message'), entry('second-q')]
  // Folding an answer appends the named card at the end of the card list.
  const reordered = [{ id: 'second-q' }, { id: 'first-q' }]
  assert.deepEqual(card_position(reordered, entries).map((card) => card.id), ['first-q', 'second-q'])
})

test('a card keeps its place while later messages are added after it', () => {
  const early = entry('question')
  const later = [entry('agent-1'), entry('agent-2'), entry('agent-3')]
  assert.deepEqual(card_position([{ id: 'question' }], [early, ...later]).map((c) => c.id), ['question'])
  // The card is still first when the answer comes back and the list was rebuilt from the end.
  assert.deepEqual(card_position([...later.map((e) => ({ id: e.id })), { id: 'question' }], [early, ...later]).map((c) => c.id), ['question', 'agent-1', 'agent-2', 'agent-3'])
})

test('a duplicated entry anchors the card to its first row', () => {
  const entries = [entry('q'), entry('something'), entry('q')]
  const cards = [{ id: 'other' }, { id: 'q' }]
  assert.deepEqual(card_position(cards, entries).map((c) => c.id), ['q', 'other'])
})

test('cards without an entry keep their incoming order at the end', () => {
  const entries = [entry('q')]
  const cards = [{ id: 'unknown-1' }, { id: 'q' }, { id: 'unknown-2' }]
  assert.deepEqual(card_position(cards, entries).map((c) => c.id), ['q', 'unknown-1', 'unknown-2'])
})

test('an empty or single card list keeps its contents', () => {
  assert.deepEqual(card_position([], [entry('q')]), [])
  assert.deepEqual(card_position([{ id: 'q' }], []), [{ id: 'q' }])
})
