import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { ServerLibraryItem } from '@shared/artifacts'
import { serverLibraryItems } from './server-library.ts'

const file = (id: string, name: string, at: number, image: boolean): ServerLibraryItem => ({
  kind: 'file',
  at,
  entryId: `e-${id}`,
  file: { id, name, size: 1, sha256: '0'.repeat(64) },
  botId: 'bot',
  botName: 'Bot',
  image,
})

const artifact = (id: string, at: number): ServerLibraryItem => ({
  kind: 'artifact',
  at,
  artifact: { id, botId: 'bot', title: id, size: 1, sha256: '0'.repeat(64), createdAt: 0, updatedAt: at },
  botId: 'bot',
  botName: 'Bot',
  image: false,
})

test('the server Library filters images and files without reordering them', () => {
  const items = [file('photo', 'Trip.PNG', 4, true), artifact('report', 3), file('plan', 'plan.md', 2, false)]
  assert.deepEqual(serverLibraryItems(items, 'all').map((item) => item.at), [4, 3, 2])
  assert.deepEqual(serverLibraryItems(items, 'images').map((item) => item.at), [4])
  assert.deepEqual(serverLibraryItems(items, 'files').map((item) => item.at), [3, 2], 'artifacts count as files')
  assert.deepEqual(serverLibraryItems([], 'images'), [])
})
