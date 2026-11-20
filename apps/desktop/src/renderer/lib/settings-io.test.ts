import assert from 'node:assert/strict'
import { test } from 'node:test'
import { exportSettings, normalizeHex, parseSettings, SETTINGS_GROUPS, SETTINGS_IO_VERSION, type SettingsGroup } from './settings-io.ts'

const now = new Date('2026-11-20T12:00:00.000Z')
const all = SETTINGS_GROUPS.map((g) => g.id)

const snapshot = {
  theme: { appearance: 'dark' },
  roster: { rosterViewFilter: { show: 'agents', unread: true, working: false, attention: false, pinned: false, sections: ['Work'], sort: 'recent' } },
  bubbles: { yourBubbles: 'tint' },
  motion: { movingAgentFaces: false, backgroundBlur: true },
  voice: { inputMode: 'pushToTalk', talkKey: 'ctrlOption' },
  appearance: { accentColor: '#1084FE' },
}

test('device prefs survive an export and import round-trip', () => {
  const json = exportSettings(snapshot, all, now)
  const file = JSON.parse(json)
  assert.equal(file.app, 'codync-settings')
  assert.equal(file.version, SETTINGS_IO_VERSION)
  assert.equal(file.exportedAt, now.toISOString())

  const result = parseSettings(json)
  assert.equal(result.ok, true)
  if (!result.ok) return
  assert.deepEqual(result.groups, snapshot)
  assert.deepEqual(result.skipped, [])
})

test('a person can export only the groups they chose', () => {
  const result = parseSettings(exportSettings(snapshot, ['bubbles', 'voice'], now))
  assert.equal(result.ok, true)
  if (!result.ok) return
  assert.deepEqual(Object.keys(result.groups).sort(), ['bubbles', 'voice'])
})

test('a file from another version is rejected clearly', () => {
  const foreign = JSON.stringify({ app: 'codync-settings', version: SETTINGS_IO_VERSION + 1, exportedAt: now.toISOString(), groups: snapshot })
  const result = parseSettings(foreign)
  assert.equal(result.ok, false)
  if (result.ok) return
  assert.match(result.error, /settings format/)
})

test('a broken file and a foreign app are rejected', () => {
  assert.equal(parseSettings('{').ok, false)
  assert.equal(parseSettings(JSON.stringify({ app: 'other-app', version: SETTINGS_IO_VERSION, groups: {} })).ok, false)
})

test('an unusable group is skipped and the others still apply', () => {
  const withBad = JSON.parse(exportSettings(snapshot, all, now))
  withBad.groups.voice = { inputMode: 'telepathy' }
  const result = parseSettings(JSON.stringify(withBad))
  assert.equal(result.ok, true)
  if (!result.ok) return
  assert.equal('voice' in result.groups, false)
  assert.equal(result.skipped.some((s) => s.id === 'voice'), true)
  assert.deepEqual(result.groups.theme, { appearance: 'dark' })
})

test('secrets and unknown fields are never exported or imported', () => {
  const source = {
    ...snapshot,
    voice: { ...snapshot.voice, apiKey: 'sk-very-secret', voiceKey: 'token' },
    appearance: { ...snapshot.appearance, account: 'someone@example.com' },
    account: { token: 'secret' },
    host: { signKey: 'secret' },
  } as unknown as Parameters<typeof exportSettings>[0]
  const json = exportSettings(source, [...all, 'account' as SettingsGroup, 'host' as SettingsGroup], now)
  for (const word of ['apiKey', 'voiceKey', 'token', 'account', 'signKey', 'someone@example.com', 'secret']) {
    assert.equal(json.includes(word), false, `export leaked ${word}`)
  }

  const foreign = JSON.parse(json)
  foreign.groups.voice.apiKey = 'sk-from-a-file'
  foreign.groups.voice.inputMode = 'pushToTalk'
  const result = parseSettings(JSON.stringify(foreign))
  assert.equal(result.ok, true)
  if (!result.ok) return
  assert.deepEqual(Object.keys(result.groups.voice!).sort(), ['inputMode', 'talkKey'])
})

test('hex input accepts 3 or 6 digits with or without a hash and rejects the rest', () => {
  assert.equal(normalizeHex('1084fe'), '#1084FE')
  assert.equal(normalizeHex(' #abc '), '#AABBCC')
  assert.equal(normalizeHex('not-a-color'), null)
  assert.equal(normalizeHex('#12345'), null)
  assert.equal(normalizeHex(123), null)
})
