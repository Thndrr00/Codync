import assert from 'node:assert/strict'
import { test } from 'node:test'
import { fontFamilies } from './font-families.ts'

test('macOS font names come from the system profiler JSON, without their metadata', () => {
  const output = JSON.stringify({ SPFontsDataType: [
    { _name: 'SF Pro Text', _kind: 'NSFont' },
    { _name: 'JetBrains Mono', _kind: 'NSFont' },
  ] })
  assert.deepEqual(fontFamilies(output, 'darwin'), ['JetBrains Mono', 'SF Pro Text'])
  assert.deepEqual(fontFamilies('not json', 'darwin'), [])
})

test('Linux and Windows font lines are split on commas and deduplicated', () => {
  assert.deepEqual(fontFamilies('DejaVu Sans, DejaVu Sans Mono\nInter, Inter\n', 'linux'), ['DejaVu Sans', 'DejaVu Sans Mono', 'Inter'])
  assert.deepEqual(fontFamilies('Consolas\r\nSegoe UI\r\n', 'win32'), ['Consolas', 'Segoe UI'])
})

test('a family name with CSS syntax is never sent to a picker', () => {
  assert.deepEqual(fontFamilies('evil;}body{color:red\nInter\n.', 'linux'), ['Inter'])
})
