import assert from 'node:assert/strict'
import { test } from 'node:test'
import {
  MAX_OPEN_TABS,
  capTabs,
  chatKey,
  restoreSavedTabs,
  safeArtifactTab,
  safeFileTab,
  type ChatRef,
  type SavedChatTabs,
  type SavedTabsByChat,
  type SavedTab,
} from './open-tabs.ts'

const chat: ChatRef = { computerId: 'mac-1', botId: 'bot-1' }
const file = {
  entryId: 'entry-1',
  entry: { id: 'entry-1', botId: 'bot-1' },
  file: { id: 'file-1', sha256: 'a'.repeat(64) },
}
const artifact: SavedTab = { kind: 'artifact', artifactId: 'art-1' }
const fileTab = safeFileTab(file, chat)!

function saved(tabs: SavedTab[], active: SavedTab | null = tabs[tabs.length - 1] ?? null): SavedChatTabs {
  return { window: 'win-1', ...chat, tabs, active }
}

test('only ids from this chat are safe to save', () => {
  assert.deepEqual(safeFileTab(file, chat), { kind: 'file', entryId: 'entry-1', fileId: 'file-1' })
  assert.equal(safeFileTab(file, { ...chat, botId: 'other-bot' }), null, 'another chat\'s entry is foreign data')
  assert.equal(safeFileTab({ ...file, entryId: 'entry-2' }, chat), null, 'the open id must match the entry')
  assert.equal(safeFileTab({ ...file, file: { id: 'file-1', sha256: 'not-a-digest' } }, chat), null)
  assert.equal(safeFileTab({ ...file, file: { id: '../secrets', sha256: 'a'.repeat(64) } }, chat), null, 'paths are not ids')
  assert.equal(safeFileTab({ ...file, entryId: 'token with spaces' }, chat), null)
  assert.deepEqual(safeArtifactTab('art-1'), artifact)
  assert.equal(safeArtifactTab('https://example.com/?token=secret'), null, 'an address is never saved')
  assert.equal(safeArtifactTab(null), null)
  assert.equal(safeArtifactTab('x'.repeat(201)), null)
})

test('a tab list drops repeats and keeps the newest capped set', () => {
  const repeated: SavedTab[] = [fileTab, artifact, fileTab, { kind: 'artifact', artifactId: 'art-2' }]
  assert.deepEqual(capTabs(repeated), [artifact, fileTab, { kind: 'artifact', artifactId: 'art-2' }], 'a reopened tab takes the newest place')
  const many: SavedTab[] = Array.from({ length: MAX_OPEN_TABS + 4 }, (_, index) => ({
    kind: 'artifact', artifactId: `art-${index}`,
  }))
  assert.equal(capTabs(many).length, MAX_OPEN_TABS)
  assert.deepEqual(capTabs(many)[0], { kind: 'artifact', artifactId: 'art-4' }, 'the oldest tabs go first')
})

test('tabs restore only with the switch on and from this chat, computer and window', () => {
  const byChat = { [chatKey(chat)]: saved([fileTab, artifact], artifact) }
  assert.deepEqual(restoreSavedTabs(byChat, chat, 'win-1', true), saved([fileTab, artifact], artifact))
  assert.equal(restoreSavedTabs(byChat, chat, 'win-1', false), null, 'the switch off restores nothing')
  assert.equal(restoreSavedTabs(byChat, chat, 'win-2', true), null, 'another window\'s tabs stay there')
  assert.equal(restoreSavedTabs(byChat, { ...chat, computerId: 'mac-2' }, 'win-1', true), null, 'another computer\'s files stay there')
  assert.equal(restoreSavedTabs(byChat, { ...chat, botId: 'bot-2' }, 'win-1', true), null, 'another chat\'s files stay there')
  assert.equal(restoreSavedTabs({}, chat, 'win-1', true), null)
})

test('a malformed or stale active tab falls back to the last saved one', () => {
  const entry = saved([fileTab, artifact], { kind: 'artifact', artifactId: 'gone' })
  assert.deepEqual(restoreSavedTabs({ [chatKey(chat)]: entry }, chat, 'win-1', true)?.active, artifact)
  const unsafe = { ...entry, tabs: [{ kind: 'artifact', artifactId: '../secret' }, fileTab] }
  assert.deepEqual(restoreSavedTabs({ [chatKey(chat)]: unsafe } as unknown as SavedTabsByChat, chat, 'win-1', true)?.tabs, [fileTab])
  assert.equal(restoreSavedTabs({ [chatKey(chat)]: { ...entry, tabs: 'bad' } } as unknown as SavedTabsByChat, chat, 'win-1', true), null)
})
