import assert from 'node:assert/strict'
import { test } from 'node:test'
import { workspace_row } from './workspace-row.ts'

test('a bot workspace row names the folder it will reveal', () => {
  assert.deepEqual(workspace_row('/Users/me/my bot'), { path: '/Users/me/my bot', label: 'my bot' })
  assert.deepEqual(workspace_row('C:\\Users\\me\\project\\'), { path: 'C:\\Users\\me\\project', label: 'project' })
  assert.deepEqual(workspace_row('\\\\server\\share\\work'), { path: '\\\\server\\share\\work', label: 'work' })
})

test('a relative, empty or traversing path is never offered as a workspace', () => {
  for (const bad of ['', '   ', 'project', '../project', '/home/me/../secret', null, 4]) {
    assert.equal(workspace_row(bad), null, `refused: ${bad}`)
  }
})

test('a root folder falls back to a readable label', () => {
  assert.deepEqual(workspace_row('C:\\'), { path: 'C:\\', label: 'Workspace' })
})
