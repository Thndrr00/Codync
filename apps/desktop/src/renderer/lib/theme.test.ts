import { test } from 'node:test'
import assert from 'node:assert/strict'
import { AVATAR_COLORS, contrastRatio, inkOn, readableFill, withAlpha } from './bubble-colors.ts'

test('solid bubbles keep their ink readable on every avatar colour', () => {
  for (const swatch of AVATAR_COLORS) {
    const fill = readableFill(swatch.hex)
    const ink = inkOn(fill)
    assert.ok(contrastRatio(ink, fill) >= 4.5, `${swatch.hex} reads at 4.5:1 or more`)
  }
})

test('a fill that already reads keeps its colour, and a borderline one moves away from the ink', () => {
  assert.equal(readableFill('#141414'), '#141414', 'a dark fill with white ink is unchanged')
  assert.notEqual(readableFill('#7c7c7c'), '#7c7c7c', 'a grey that fails 4.5:1 with both inks is nudged')
  assert.ok(contrastRatio(inkOn(readableFill('#7c7c7c')), readableFill('#7c7c7c')) >= 4.5)
})

test('the contrast ratio is symmetric and the ink follows the darker side', () => {
  assert.equal(contrastRatio('#ffffff', '#000000'), contrastRatio('#000000', '#ffffff'))
  assert.equal(inkOn('#101010'), '#ffffff')
  assert.equal(inkOn('#f8f8f8'), '#141414')
})

test('an alpha suffix keeps the colour', () => {
  assert.equal(withAlpha('#3b82f6', 0.5), 'rgba(59, 130, 246, 0.5)')
})
