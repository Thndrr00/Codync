import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { SpeechEvent } from '../../../shared/ipc.ts'
import { activeDictation } from './active-dictation.ts'
import { Dictation, stopDictation, type DictationPlatform } from './dictation.ts'
import { MIN_SAMPLES } from './dictation-pieces.ts'

function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((yes) => { resolve = yes })
  return { promise, resolve }
}

function setup(overrides: Partial<DictationPlatform> = {}) {
  let listener: ((event: SpeechEvent) => void) | null = null
  let feed: ((pcm: Int16Array, level: number) => void) | null = null
  let box = 'PRIVATE_DRAFT'
  const log: string[] = []
  const pending: ((text: string) => void)[] = []
  const platform: DictationPlatform = {
    speech: {
      start: () => { log.push('speech.start') },
      stop: () => { log.push('speech.stop') },
      onEvent: (callback) => { listener = callback; return () => { listener = null } },
    },
    noSpeech: 'No speech here.', host: true,
    language: () => null, provider: () => null,
    checkKey: async () => true,
    configureKey: () => { log.push('configure') },
    microphone: async (out) => { feed = out; log.push('mic.open'); return () => { log.push('mic.close') } },
    transcribe: () => new Promise((resolve) => { pending.push(resolve) }),
    count: () => { log.push('count') },
    report: () => { log.push('report') },
    ...overrides,
  }
  const dictation = new Dictation((change) => { box = change(box) }, platform)
  return { dictation, log, pending, box: () => box, say: (event: SpeechEvent) => listener?.(event), hear: () => feed?.(new Int16Array(MIN_SAMPLES * 2), 0.5) }
}

const settle = () => new Promise((resolve) => setTimeout(resolve, 0))
function metadataOnly() {
  assert.deepEqual(Object.keys(activeDictation.snapshot!).sort(), ['id', 'phase', 'startedAt'])
  assert.equal(JSON.stringify(activeDictation.snapshot).includes('PRIVATE'), false)
}

test('dictation publishes starting while key readiness is pending and listening only after microphone acquisition', async (t) => {
  t.after(() => stopDictation())
  const key = deferred<boolean>()
  const s = setup({ provider: () => 'openai', checkKey: () => key.promise })
  s.dictation.start(s.box())
  const identity = activeDictation.snapshot!.id
  assert.equal(activeDictation.snapshot?.phase, 'starting')
  assert.equal(activeDictation.snapshot?.startedAt, null)
  assert.deepEqual(s.log, [])
  metadataOnly()
  key.resolve(true)
  await settle()
  assert.equal(activeDictation.snapshot?.id, identity)
  assert.equal(activeDictation.snapshot?.phase, 'listening')
  assert.ok(activeDictation.snapshot!.startedAt! > 0)
  assert.deepEqual(s.log, ['mic.open'])
  metadataOnly()
  stopDictation(identity)
  assert.equal(activeDictation.snapshot, null)
  assert.deepEqual(s.log, ['mic.open', 'mic.close'])
})

test('provider transcription publishes metadata while its private transcript remains with the composer', async (t) => {
  t.after(() => stopDictation())
  const s = setup({ provider: () => 'gemini' })
  s.dictation.start(s.box())
  await settle()
  const identity = activeDictation.snapshot!.id
  const startedAt = activeDictation.snapshot!.startedAt
  s.hear()
  s.dictation.finish()
  assert.equal(activeDictation.snapshot?.phase, 'transcribing')
  assert.equal(activeDictation.snapshot?.startedAt, startedAt)
  metadataOnly()
  await settle()
  assert.equal(s.pending.length, 1)
  s.pending[0]!('PRIVATE_TRANSCRIPT')
  await settle()
  assert.equal(activeDictation.snapshot, null)
  assert.match(s.box(), /PRIVATE_TRANSCRIPT/)
  stopDictation(identity)
  assert.equal(activeDictation.snapshot, null)
})

test('on-device finish waits in transcribing state until the recognizer stops', (t) => {
  t.after(() => stopDictation())
  const s = setup()
  s.dictation.start(s.box())
  const identity = activeDictation.snapshot!.id
  assert.equal(activeDictation.snapshot?.phase, 'listening')
  s.dictation.finish()
  assert.equal(activeDictation.snapshot?.phase, 'transcribing')
  s.say({ type: 'final', text: 'PRIVATE_RECOGNIZED_WORDS' })
  assert.equal(activeDictation.snapshot?.id, identity)
  metadataOnly()
  s.say({ type: 'stopped' })
  assert.equal(activeDictation.snapshot, null)
})

test('a stale stop identity cannot cancel a replacement and the legacy stop still cancels the current owner', (t) => {
  t.after(() => stopDictation())
  const previous = setup()
  const next = setup()
  previous.dictation.start(previous.box())
  const previousId = activeDictation.snapshot!.id
  next.dictation.start(next.box())
  const nextId = activeDictation.snapshot!.id
  assert.notEqual(nextId, previousId)
  assert.equal(previous.dictation.busy, false)
  stopDictation(previousId)
  assert.equal(next.dictation.listening, true)
  assert.equal(activeDictation.snapshot?.id, nextId)
  assert.deepEqual(next.log, ['speech.start'])
  stopDictation()
  assert.equal(next.dictation.busy, false)
  assert.equal(activeDictation.snapshot, null)
  assert.deepEqual(next.log, ['speech.start', 'speech.stop'])
})

test('a late previous transcript cannot clear or edit a replacement session', async (t) => {
  t.after(() => stopDictation())
  const previous = setup({ provider: () => 'openai' })
  previous.dictation.start(previous.box())
  await settle()
  previous.hear()
  previous.dictation.finish()
  await settle()
  const next = setup()
  next.dictation.start(next.box())
  const nextId = activeDictation.snapshot!.id
  previous.pending[0]!('PRIVATE_LATE_TEXT')
  await settle()
  assert.equal(activeDictation.snapshot?.id, nextId)
  assert.equal(activeDictation.snapshot?.phase, 'listening')
  assert.equal(next.dictation.busy, true)
  assert.equal(previous.box(), 'PRIVATE_DRAFT')
  assert.equal(next.box(), 'PRIVATE_DRAFT')
})

test('missing speech support or provider credentials clears starting metadata without opening a microphone', async (t) => {
  t.after(() => stopDictation())
  const noSpeech = setup({ speech: null })
  noSpeech.dictation.start(noSpeech.box())
  assert.equal(activeDictation.snapshot, null)
  assert.equal(noSpeech.dictation.busy, false)
  assert.equal(noSpeech.log.includes('mic.open'), false)
  const noKey = setup({ provider: () => 'openai', checkKey: async () => false })
  noKey.dictation.start(noKey.box())
  await settle()
  assert.equal(activeDictation.snapshot, null)
  assert.deepEqual(noKey.log, ['configure'])
})

test('published metadata is copied and an old clear cannot erase another identity', (t) => {
  t.after(() => { const id = activeDictation.snapshot?.id; if (id) activeDictation.clear(id) })
  const snapshot = { id: 'old', phase: 'starting' as const, startedAt: null }
  activeDictation.publish(snapshot)
  snapshot.id = 'mutated'
  assert.equal(activeDictation.snapshot?.id, 'old')
  activeDictation.publish({ id: 'new', phase: 'listening', startedAt: 123 })
  activeDictation.clear('old')
  assert.equal(activeDictation.snapshot?.id, 'new')
  activeDictation.clear('new')
  assert.equal(activeDictation.snapshot, null)
})
