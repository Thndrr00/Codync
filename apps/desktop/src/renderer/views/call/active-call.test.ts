import assert from 'node:assert/strict'
import { test } from 'node:test'
import { activeCall, type ActiveCallSnapshot } from './active-call.ts'

const snapshot = (ref = 'computer/bot'): ActiveCallSnapshot => ({ id: `${ref}-call`, ref, name: ref, phase: 'listening', muted: false })

test('call controls dispatch only while a session is registered', () => {
  const invoked: string[] = []
  assert.equal(activeCall.snapshot, null)
  assert.equal(activeCall.invoke('mute'), false)
  assert.equal(activeCall.invoke('end'), false)
  const release = activeCall.publish(snapshot(), {
    mute: () => invoked.push('mute'),
    end: () => invoked.push('end'),
  })
  try {
    assert.equal(activeCall.invoke('mute'), true)
    assert.equal(activeCall.invoke('end'), true)
    assert.deepEqual(invoked, ['mute', 'end'])
  } finally {
    release()
  }
  assert.equal(activeCall.snapshot, null)
  assert.equal(activeCall.invoke('mute'), false)
  assert.equal(activeCall.invoke('end'), false)
  assert.deepEqual(invoked, ['mute', 'end'])
})

test('published call state is copied rather than sharing its mutable input', () => {
  const source = snapshot()
  const expected = { ...source }
  const release = activeCall.publish(source, { mute: () => {}, end: () => {} })
  try {
    source.ref = 'other/computer'
    source.name = 'Changed'
    source.phase = 'failed'
    source.muted = true
    assert.deepEqual(activeCall.snapshot, expected)
    assert.notEqual(activeCall.snapshot, source)
  } finally {
    release()
  }
})

test('an old effect cleanup cannot erase or control a newer active call', () => {
  const invoked: string[] = []
  const first = activeCall.publish(snapshot('computer/first'), {
    mute: () => invoked.push('first mute'), end: () => invoked.push('first end'),
  })
  const second = activeCall.publish(snapshot('computer/second'), {
    mute: () => invoked.push('second mute'), end: () => invoked.push('second end'),
  })
  try {
    first()
    first()
    assert.equal(activeCall.snapshot?.ref, 'computer/second')
    assert.equal(activeCall.invoke('mute'), true)
    assert.equal(activeCall.invoke('end'), true)
    assert.deepEqual(invoked, ['second mute', 'second end'])
  } finally {
    second()
    first()
  }
  assert.equal(activeCall.snapshot, null)
  assert.equal(activeCall.invoke('end'), false)
})
