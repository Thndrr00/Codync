import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { AppModel } from '../../store/app-model'
import { islandSignals } from '../../store/island-signals.ts'
import { activeDictation } from './active-dictation.ts'

test('upload signals preserve zero progress and distinguish uploads from pending sends without file bytes', () => {
  const entries = [{ kind: 'user', data: { status: 'sending', text: 'PRIVATE_MESSAGE', attachments: [{ id: 'zero', name: 'upload.txt', data: 'PRIVATE_BYTES' }, { id: 'pending', name: 'pending.txt' }] } }, { kind: 'user', data: { status: 'completed', attachments: [{ id: 'done', name: 'finished.txt' }] } }]
  const store = { computer: { id: 'pc', name: 'Computer' }, connection: { kind: 'online' }, lastError: null, uploadPercents: new Map([['zero', 0]]), allEntries: () => entries }
  const app = { stores: new Map([['pc', store]]), roster: [{ bot: { id: 'bot' }, ref: { computerId: 'pc', botId: 'bot' }, store }] } as unknown as AppModel
  const snapshot = islandSignals(app)
  assert.deepEqual(snapshot.uploads, [{ ref: 'pc/bot', name: 'upload.txt', percent: 0, status: 'Uploading' }, { ref: 'pc/bot', name: 'pending.txt', percent: null, status: 'Sending' }])
  assert.equal(JSON.stringify(snapshot).includes('PRIVATE'), false)
})

test('connection and dictation signals report existing owner state without starting a recognizer', () => {
  const store = { computer: { id: 'remote', name: 'Remote' }, connection: { kind: 'offline' }, lastError: 'Disconnected' }
  const app = { stores: new Map([['remote', store]]), roster: [] } as unknown as AppModel
  activeDictation.publish({ id: 'existing', phase: 'transcribing', startedAt: 123 })
  const snapshot = islandSignals(app)
  assert.deepEqual(snapshot.computers, [{ id: 'remote', name: 'Remote', state: 'offline', error: 'Disconnected' }])
  assert.deepEqual(snapshot.dictation, { id: 'existing', phase: 'transcribing', startedAt: 123 })
  activeDictation.clear('existing')
})
