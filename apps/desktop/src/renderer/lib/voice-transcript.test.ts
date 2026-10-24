import { test } from 'node:test'
import assert from 'node:assert/strict'
import { CallTranscript } from './voice-transcript.ts'

test('provider transcripts are deduplicated without claiming playback or confirmation', () => {
  const capture = new CallTranscript()
  const event = { type: 'conversation.item.input_audio_transcription.completed', item_id: 'a', content_index: 0, transcript: 'Turn on the light' }
  capture.openAI(event); capture.openAI(event)
  capture.openAI({ ...event, type: 'response.output_audio_transcript.done', item_id: 'b', transcript: 'The light is on' })
  capture.interrupted()
  assert.deepEqual(capture.snapshot(), { entries: [
    { role: 'user', source: 'recognized', text: 'Turn on the light' },
    { role: 'assistant', source: 'generated', text: 'The light is on' },
  ], truncated: false, interruptions: 1 })
})

test('Gemini deltas keep whitespace and separate provider turns', () => {
  const capture = new CallTranscript()
  capture.gemini({ inputTranscription: { text: 'hello' }, outputTranscription: { text: 'good' } })
  capture.gemini({ outputTranscription: { text: ' ' } })
  capture.gemini({ outputTranscription: { text: 'morning' }, interrupted: true, turnComplete: true })
  capture.gemini({ outputTranscription: { text: 'Next response' }, turnComplete: true })
  const result = capture.snapshot()
  assert.deepEqual(result.entries.map((row) => row.text), ['hello', 'good morning', 'Next response'])
  assert.equal(result.interruptions, 1)
})

test('capture is bounded by segment count and text budget with truthful truncation', () => {
  const capture = new CallTranscript()
  for (let i = 0; i < 10; i++) capture.add('assistant', 'queued', 'x'.repeat(5000))
  assert.equal(capture.snapshot().entries.reduce((sum, row) => sum + row.text.length, 0), 32000)
  assert.equal(capture.snapshot().truncated, true)
  const many = new CallTranscript()
  for (let i = 0; i < 100; i++) many.add('user', 'recognized', 'a')
  assert.equal(many.snapshot().entries.length, 96)
  assert.equal(many.snapshot().truncated, true)
})

test('snapshots are detached and truncation preserves Unicode scalar boundaries', () => {
  const capture = new CallTranscript()
  capture.add('user', 'recognized', 'a'.repeat(3999) + '😀', 'a')
  const result = capture.snapshot()
  assert.equal(result.entries[0]?.text.length, 3999)
  result.entries[0]!.text = 'changed externally'
  assert.equal(capture.snapshot().entries[0]?.text.length, 3999)
  assert.equal(capture.snapshot().truncated, true)
})

test('malformed and unrelated events cannot create transcript text', () => {
  const capture = new CallTranscript()
  capture.openAI({ type: 'response.done', transcript: 'invented', item_id: 'x', content_index: 0 })
  capture.openAI({ type: 'response.output_audio_transcript.done', transcript: 'missing identity' })
  capture.gemini({ inputTranscription: null, outputTranscription: { text: 42 } })
  capture.add('user', 'recognized', '  ')
  assert.deepEqual(capture.snapshot().entries, [])
})

test('invalid NUL text is omitted without losing the receipt', () => {
  const capture = new CallTranscript()
  capture.add('user', 'recognized', '\0')
  capture.add('user', 'recognized', 'hello\0 there', 'user')
  capture.add('user', 'recognized', '\0!', 'user', true)
  assert.deepEqual(capture.snapshot(), {
    entries: [{ role: 'user', source: 'recognized', text: 'hello there!' }],
    truncated: true, interruptions: 0,
  })
})
