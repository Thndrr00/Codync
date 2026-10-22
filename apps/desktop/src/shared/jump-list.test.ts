import assert from 'node:assert/strict'
import { test } from 'node:test'
import { botRefFromArgv, jumpListBots, openBotArgument } from './jump-list.ts'

const bot = (ref: string, name: string, pinned: boolean) => ({ ref, name, pinned })

test('only named pinned bots reach the jump list, up to the limit', () => {
  const bots = [bot('c/a', 'Alpha', true), bot('c/b', 'Beta', false), bot('c/c', ' ', true), bot('c/d', 'Delta', true), bot('c/e', 'Echo', true)]
  assert.deepEqual(jumpListBots(bots, 2).map((b) => b.ref), ['c/a', 'c/d'])
  assert.deepEqual(jumpListBots(bots, 0), [])
})

test('an open-bot argument round-trips through the command line', () => {
  const ref = `${'A'.repeat(22)}/bot-1`
  assert.equal(botRefFromArgv(['app.exe', openBotArgument(ref)]), ref)
})

test('malformed bot references on the command line are ignored', () => {
  assert.equal(botRefFromArgv(['app.exe']), null)
  assert.equal(botRefFromArgv(['app.exe', '--open-bot=no-slash']), null)
  assert.equal(botRefFromArgv(['app.exe', '--open-bot=a/b/c']), null)
  assert.equal(botRefFromArgv(['app.exe', '--open-bot=a/b c']), null)
})
