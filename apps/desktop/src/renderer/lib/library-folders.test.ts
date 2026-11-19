import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { LibraryItem } from '@shared/artifacts'
import { inSection, libraryItemKey, visibleFolders, type LibraryFolder } from './library-folders.ts'

const DAY = 24 * 60 * 60 * 1000
const artifact = (id: string, folderId?: string, at = 10 * DAY): LibraryItem => ({
  kind: 'artifact', at, folderId, artifact: { id, botId: 'b', title: id, size: 0, sha256: '', createdAt: at, updatedAt: at },
})

const folders: LibraryFolder[] = [
  { id: 'x', name: 'Reports', at: 90, count: 1, items: ['artifact:a'] },
  { id: 'y', name: 'Drafts', at: 80, count: 1, items: ['artifact:b'] },
]

test('an item key matches the host key for every kind', () => {
  assert.equal(libraryItemKey(artifact('a')), 'artifact:a')
  assert.equal(
    libraryItemKey({ kind: 'file', at: 1, entryId: 'e', file: { id: 'f', name: 'f', size: 0, sha256: '' } }),
    'file:e:f',
  )
})

test('each side-rail view selects the right items', () => {
  const now = 10 * DAY
  const a = artifact('a', 'x', now - DAY)
  const b = artifact('b', 'y', now - 9 * DAY)
  const loose = artifact('c', undefined, now - 10 * DAY)
  assert.deepEqual([a, b, loose].filter((i) => inSection(i, 'all', [], now)), [a, b, loose])
  assert.deepEqual([a, b, loose].filter((i) => inSection(i, 'recent', [], now)), [a])
  assert.deepEqual([a, b, loose].filter((i) => inSection(i, 'unfiled', [], now)), [loose])
  assert.deepEqual([a, b, loose].filter((i) => inSection(i, 'favorites', ['y'], now)), [b])
  assert.deepEqual([a, b, loose].filter((i) => inSection(i, 'x', [], now)), [a])
})

test('folder tiles follow the selected view and favorites', () => {
  assert.deepEqual(visibleFolders(folders, 'all', []), folders)
  assert.deepEqual(visibleFolders(folders, 'unfiled', []), [])
  assert.deepEqual(visibleFolders(folders, 'favorites', ['y']), [folders[1]])
  assert.deepEqual(visibleFolders(folders, 'x', []), [folders[0]])
})
