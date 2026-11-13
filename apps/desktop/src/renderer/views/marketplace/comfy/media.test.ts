import assert from 'node:assert/strict'
import test from 'node:test'
import { accepts, fieldWord, pickLabel } from './media.ts'

test('a field takes the files its loader reads', () => {
  assert.equal(accepts('image', 'start.png'), true)
  assert.equal(accepts('image', 'clip.mp4'), false)
  assert.equal(accepts('video', 'source.mp4'), true)
  assert.equal(accepts('video', 'mask.webm'), true)
  assert.equal(accepts('video', 'song.wav'), false)
  assert.equal(accepts('audio', 'song.wav'), true)
  assert.equal(accepts('audio', 'song.mp3'), true)
  assert.equal(accepts('audio', 'reference.png'), false)
})

test('the picker names the kind it wants', () => {
  assert.equal(fieldWord('video'), 'video')
  assert.equal(fieldWord('audio'), 'sound')
  assert.equal(pickLabel('video', false), 'Choose a video')
  assert.equal(pickLabel('image', true), 'Choose another picture')
})
