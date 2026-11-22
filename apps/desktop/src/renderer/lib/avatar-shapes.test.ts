import assert from 'node:assert/strict'
import { test } from 'node:test'
import { AVATAR_SHAPES, isAvatarShape, safeAvatarShape } from './avatar-shapes.ts'

test('the avatar catalog draws all eleven shapes, including the three new creatures', () => {
  assert.equal(AVATAR_SHAPES.length, 11)
  for (const shape of ['ghost', 'jellyfish', 'octopus']) {
    assert.ok(AVATAR_SHAPES.includes(shape), `${shape} is in the picker`)
  }
  assert.equal(new Set(AVATAR_SHAPES).size, AVATAR_SHAPES.length)
})

test('an unknown or empty avatar shape falls back to the blob', () => {
  assert.equal(isAvatarShape('ghost'), true)
  assert.equal(isAvatarShape('monster'), false)
  assert.equal(isAvatarShape(null), false)
  assert.equal(safeAvatarShape('octopus'), 'octopus')
  assert.equal(safeAvatarShape('monster'), 'blob')
  assert.equal(safeAvatarShape(undefined), 'blob')
})
