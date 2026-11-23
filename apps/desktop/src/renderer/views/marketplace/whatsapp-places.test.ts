import assert from 'node:assert/strict'
import test from 'node:test'
import { whatsappPlaces } from './whatsapp-places.ts'

test('the owners own chat is a place even with a non-empty allowlist', () => {
  const me = '5215555555555@s.whatsapp.net'
  assert.deepEqual(whatsappPlaces(me, []), [{ id: me, own: true }])
  assert.deepEqual(whatsappPlaces(me, ['friend@s.whatsapp.net']), [
    { id: me, own: true },
    { id: 'friend@s.whatsapp.net', own: false },
  ])
})

test('an unlinked number has no own place and the allowlist is not duplicated', () => {
  assert.deepEqual(whatsappPlaces(null, ['friend@s.whatsapp.net']), [{ id: 'friend@s.whatsapp.net', own: false }])
  const me = 'me@s.whatsapp.net'
  assert.deepEqual(whatsappPlaces(me, [me, 'friend@s.whatsapp.net']), [
    { id: me, own: true },
    { id: 'friend@s.whatsapp.net', own: false },
  ])
})
