import assert from 'node:assert/strict'
import { test } from 'node:test'
import { audioExtension, audioMime, isAudioName } from './audio.ts'

test('sound files are recognised by name, case and extension', () => {
  for (const name of ['note.M4A', 'voice.wav', 'clip.aac', 'song.ogg', 'take.opus', 'master.flac', 'old.mp3', 'a.oga']) {
    assert.equal(isAudioName(name), true, name)
  }
  for (const name of ['picture.png', 'movie.mp4', 'notes.pdf', 'archive.zip', 'no-extension']) {
    assert.equal(isAudioName(name), false, name)
  }
})

test('the extension is the lower-cased last component', () => {
  assert.equal(audioExtension('archive.tar.WAV'), 'wav')
  assert.equal(audioExtension('voice'), 'voice')
  assert.equal(audioExtension(''), '')
})

test('each sound gets a MIME the browser can play, with a safe default', () => {
  assert.equal(audioMime('voice.m4a'), 'audio/mp4')
  assert.equal(audioMime('voice.wav'), 'audio/wav')
  assert.equal(audioMime('voice.aac'), 'audio/aac')
  assert.equal(audioMime('voice.ogg'), 'audio/ogg')
  assert.equal(audioMime('voice.OPUS'), 'audio/ogg')
  assert.equal(audioMime('voice.flac'), 'audio/flac')
  assert.equal(audioMime('voice.unknown'), 'audio/mpeg')
})
