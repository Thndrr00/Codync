import assert from 'node:assert/strict'
import { existsSync } from 'node:fs'
import { registerHooks } from 'node:module'
import { test } from 'node:test'

// The renderer imports without extensions, as the bundler resolves them.
registerHooks({
  resolve(specifier, context, nextResolve) {
    if (specifier.startsWith('.') && context.parentURL && !/\.[a-z]+$/.test(specifier)) {
      const url = new URL(`${specifier}.ts`, context.parentURL)
      if (existsSync(url)) return nextResolve(url.href, context)
    }
    return nextResolve(specifier, context)
  },
})

test('a call takes another engine only when what its engine is built from changes', async () => {
  const disk = new Map<string, string>()
  Object.assign(globalThis, {
    localStorage: { getItem: (key: string) => disk.get(key) ?? null, setItem: (key: string, value: string) => disk.set(key, value) },
    window: { addEventListener() {} },
  })
  const { VoiceSettings } = await import('./voice.ts')
  const settings = await import('../settings/voice-settings.ts')

  const device = VoiceSettings.engineChoice(null)
  assert.equal(VoiceSettings.engineChoice(null), device)
  settings.language.set('ja-JP')
  assert.notEqual(VoiceSettings.engineChoice(null), device)
  assert.notEqual(VoiceSettings.engineChoice('gemini'), VoiceSettings.engineChoice('openai'))

  const seen = [VoiceSettings.engineChoice('gemini')]
  for (const change of [
    () => settings.saveModel('gemini', 'realtime', 'live-9'),
    () => settings.saveVoice('gemini', 'Puck'),
    () => settings.mode('gemini').set('speech'),
    () => settings.saveModel('gemini', 'transcribe', 'flash-9'),
    () => settings.saveModel('gemini', 'speech', 'tts-9'),
  ]) {
    change()
    const choice = VoiceSettings.engineChoice('gemini')
    assert.equal(VoiceSettings.engineChoice('gemini'), choice)
    assert.ok(!seen.includes(choice), choice)
    seen.push(choice)
  }
})


test('ElevenLabs holds no live call, so its mode is always speech', async () => {
  const { VoiceSettings } = await import('./voice.ts')
  const settings = await import('../settings/voice-settings.ts')
  settings.mode('elevenlabs').set('realtime')
  assert.equal(VoiceSettings.isSpeechMode('elevenlabs'), true)
  assert.ok(VoiceSettings.engineChoice('elevenlabs').startsWith('elevenlabs speech'))
  // Its voices come from the account, so the fixed list doesn't validate them.
  settings.saveVoice('elevenlabs', '21m00Tcm4TlvDq8ikWAM')
  assert.equal(VoiceSettings.voice('elevenlabs'), '21m00Tcm4TlvDq8ikWAM')
})


test('spoken text drops bullets, task boxes, nested quotes and setext rules but keeps words', async () => {
  const { cleanSpokenText, spokenText } = await import('./voice.ts')
  const reply = [
    '## Plan',
    '> - outer bullet',
    '> > • nested bullet',
    '1. - [ ] first task',
    '   * [x] nested task',
    '+ keep C++, x*2 and #tag intact',
    'Heading',
    '===',
    'Plain prose with a quote: "ship it" and a dash - inside.',
  ].join('\n')
  assert.equal(cleanSpokenText(reply), [
    'Plan',
    'outer bullet',
    'nested bullet',
    'first task',
    'nested task',
    'keep C++, x*2 and #tag intact',
    'Heading',
    '',
    'Plain prose with a quote: "ship it" and a dash - inside.',
  ].join('\n'))
  assert.equal(spokenText('- **Bold** and [link](https://x.y/1)\n===\n\nMore.'), 'Bold and link\nMore.')
})

test('a repeated provider utterance is posted once inside the window, not after it', async () => {
  const { isRepeatUtterance } = await import('./voice.ts')
  const at = 10_000
  assert.equal(isRepeatUtterance('ship it', { text: 'ship it', at }, 2_000, 11_000), true)
  assert.equal(isRepeatUtterance('  ship it  ', { text: 'ship it', at }, 2_000, 11_000), true)
  assert.equal(isRepeatUtterance('ship it', { text: 'ship it', at }, 2_000, 12_001), false)
  assert.equal(isRepeatUtterance('ship it again', { text: 'ship it', at }, 2_000, 11_000), false)
  assert.equal(isRepeatUtterance('ship it', null, 2_000, 11_000), false)
  assert.equal(isRepeatUtterance('', { text: '', at }, 2_000, 11_000), false)
  assert.equal(isRepeatUtterance('ship it', { text: 'ship it', at: 11_500 }, 2_000, 11_000), false)
})

test('only a new explicit user message interrupts a reply being read aloud', async () => {
  const { shouldInterruptSpeech } = await import('./voice.ts')
  assert.equal(shouldInterruptSpeech('user', true), true)
  for (const source of ['channel', 'thread', 'self', 'notice'] as const) {
    assert.equal(shouldInterruptSpeech(source, true), false)
    assert.equal(shouldInterruptSpeech(source, false), false)
  }
  assert.equal(shouldInterruptSpeech('user', false), false)
})


test('Windows and Linux retain automatic call fallback until a provider is explicitly picked', async () => {
  const { VoiceSettings } = await import('./voice.ts')
  const settings = await import('../settings/voice-settings.ts')
  for (const platform of ['win32', 'linux']) {
    Object.assign(window, { codync: { platform } })
    settings.engine.set('device')
    // The picker offers a supported choice, but startup still enters configured-provider fallback.
    assert.equal(settings.engine.get(), 'openai')
    assert.equal(VoiceSettings.provider(), null)
    settings.engine.set('gemini')
    assert.equal(VoiceSettings.provider(), 'gemini')
    assert.equal(settings.engine.get(), 'gemini')
  }
})
