import assert from 'node:assert/strict'
import { test } from 'node:test'
import { isAbsolute, sep } from 'node:path'
import { revealableFolder } from './workspace-path.ts'

const folder = isAbsolute('C:\\') && sep === '\\' ? 'C:\\Users\\me\\project' : '/home/me/project'

test('a bot workspace is revealed as an absolute folder path', () => {
  assert.equal(revealableFolder(folder), folder)
  assert.equal(revealableFolder(`${folder}/`), folder)
})

test('a relative path, traversal and a non-string are refused', () => {
  // Whether an accepted path is a directory is checked in the IPC handler, not by this decision.
  for (const bad of ['project', '../project', `${folder}/../secret`, '', null, 17]) {
    assert.equal(revealableFolder(bad), null, `refused: ${bad}`)
  }
})
