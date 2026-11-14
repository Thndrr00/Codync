import assert from 'node:assert/strict'
import { test } from 'node:test'
import { firstChoice, mediaDraft, mediaPrompt, modelsOf, onModel, type MediaAnswer } from '../../../shared/media.ts'

const model = (id: string, kinds: ('image' | 'video')[], extra: Record<string, unknown> = {}) => ({
  id,
  name: id,
  maker: 'x',
  subscription: 'y',
  kinds,
  aspects: ['1:1', '16:9'],
  transparent: false,
  voices: false,
  capabilities: { aspects: ['1:1', '16:9'], durations: [], resolutions: [], transparent: false, voices: null },
  ...extra,
})

const answer: MediaAnswer = {
  agent: 'grok',
  installed: true,
  models: [
    model('grok-imagine', ['image', 'video'], {
      transparent: true,
      voices: true,
      capabilities: {
        aspects: ['1:1', '16:9'],
        durations: [6, 10],
        resolutions: ['480p', '720p'],
        transparent: true,
        voices: [{ id: 'ara', name: 'Ara' }],
      },
    }),
  ],
}

test('a kind offers only the models that make it and starts on the first', () => {
  assert.deepEqual(modelsOf(answer, 'image').map((m) => m.id), ['grok-imagine'])
  assert.equal(modelsOf({ ...answer, models: [model('gpt-image', ['image'])] }, 'video').length, 0)
  assert.equal(modelsOf(null, 'image').length, 0)
  const choice = firstChoice(answer, 'video')
  assert.deepEqual(choice, { kind: 'video', model: 'grok-imagine', aspect: '1:1', duration: 6, resolution: '480p', transparent: false, voice: null })
  assert.equal(firstChoice({ ...answer, models: [] }, 'image'), null, 'nothing to choose')
})

test('another model keeps only the settings it makes', () => {
  const choice = { kind: 'video' as const, model: 'grok-imagine', aspect: '16:9', duration: 10, resolution: '720p', transparent: true, voice: 'ara' }
  const still = model('other', ['video'])
  const moved = onModel(choice, still)
  assert.equal(moved.model, 'other')
  assert.equal(moved.aspect, '16:9', 'a frame it makes stays')
  assert.equal(moved.duration, 0, 'no length it makes falls back')
  assert.equal(moved.voice, null)
  assert.equal(moved.transparent, false)
})

test('the description after the flags is kept, and the flags are rebuilt', () => {
  assert.equal(mediaPrompt('a red fox'), 'a red fox')
  assert.equal(mediaPrompt('/image --model grok-imagine --aspect 16:9 a red fox'), 'a red fox')
  assert.equal(mediaPrompt('/video --ref /tmp/a.png --voice ara a pan'), 'a pan')
  assert.equal(mediaPrompt('/image --transparent a cup'), 'a cup')
  assert.equal(mediaPrompt('/image'), '')
  assert.equal(mediaPrompt('/'), '')
  assert.equal(mediaPrompt('/ima a cup'), 'a cup')
  const choice = { kind: 'image' as const, model: 'gpt-image', aspect: '4:3', duration: 0, resolution: '', transparent: true, voice: null }
  assert.equal(mediaDraft(choice, '/image a cup'), '/image --model gpt-image --aspect 4:3 --transparent a cup')
  assert.equal(mediaDraft(choice, '/'), '/image --model gpt-image --aspect 4:3 --transparent ')
  const video = { ...choice, kind: 'video' as const, duration: 10, resolution: '720p', voice: 'ara' }
  assert.equal(mediaDraft(video, ''), '/video --model gpt-image --aspect 4:3 --duration 10 --resolution 720p --voice ara ')
})
