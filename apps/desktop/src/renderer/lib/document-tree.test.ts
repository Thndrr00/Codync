import { test } from 'node:test'
import assert from 'node:assert/strict'
import { documentTree, documentDescendants } from './document-tree.ts'

const pages = [
  { id: 'child', parentId: 'parent', title: 'Child' },
  { id: 'parent', parentId: null, title: 'Parent' },
  { id: 'grandchild', parentId: 'child', title: 'Grandchild' },
  { id: 'missing', parentId: 'not-loaded', title: 'Missing parent' },
]
test('document navigation groups subpages and keeps missing ancestors discoverable', () => {
  assert.deepEqual(documentTree(pages).map(({ page, depth }) => [page.id, depth]), [['parent', 0], ['child', 1], ['grandchild', 2], ['missing', 0]])
  assert.ok(documentTree(pages, true).every(({ depth }) => depth === 0))
})
test('move targets exclude all loaded descendants and malformed cycles do not loop', () => {
  assert.deepEqual([...documentDescendants(pages, 'parent')], ['parent', 'child', 'grandchild'])
  assert.equal(documentTree([{ id: 'a', parentId: 'b', title: '' }, { id: 'b', parentId: 'a', title: '' }]).length, 2)
})
