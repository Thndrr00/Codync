import assert from 'node:assert/strict'
import { test } from 'node:test'
import { attachmentSymbol, byteCount, isAudioName, isMeshName, isVideoName, mediaMime } from './media.ts'

test('the chat tells each ComfyUI output apart by its extension', () => {
  assert.equal(isVideoName('clip.MP4'), true)
  assert.equal(isVideoName('clip.webm'), true)
  assert.equal(isAudioName('take.wav'), true)
  assert.equal(isAudioName('take.opus'), true)
  assert.equal(isMeshName('scene.glb'), true)
  assert.equal(isMeshName('scene.splat'), true)
  assert.equal(isVideoName('picture.png'), false)
  assert.equal(isAudioName('clip.mp4'), false)
  assert.equal(isMeshName('clip.mp4'), false)
})

test('a video or audio blob gets the MIME that picks its decoder', () => {
  assert.equal(mediaMime('clip.mp4'), 'video/mp4')
  assert.equal(mediaMime('clip.WebM'), 'video/webm')
  assert.equal(mediaMime('take.wav'), 'audio/wav')
  assert.equal(mediaMime('take.ogg'), 'audio/ogg')
  assert.equal(mediaMime('scene.glb'), '')
  assert.equal(mediaMime('picture.png'), '')
})

test('the card symbol follows the same families', () => {
  assert.equal(attachmentSymbol('clip.mp4'), 'film')
  assert.equal(attachmentSymbol('take.flac'), 'waveform')
  assert.equal(attachmentSymbol('scene.glb'), 'shippingbox')
  assert.equal(attachmentSymbol('notes.txt'), 'doc')
  assert.equal(attachmentSymbol('sheet.pdf'), 'doc.richtext')
})

test('a size is readable and keeps the chat consistent', () => {
  assert.equal(byteCount(999), '999 bytes')
  assert.equal(byteCount(1500), '1.5 KB')
  assert.equal(byteCount(2_500_000), '2.5 MB')
  assert.equal(byteCount(31_200_000_000), '31 GB')
})
