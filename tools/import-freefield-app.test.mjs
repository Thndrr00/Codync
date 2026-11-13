import assert from 'node:assert/strict'
import { test } from 'node:test'
import { convert } from './import-freefield-app.mjs'

const app = {
  id: 'demo',
  name: 'Demo',
  section: 'image',
  summary: 'A demo app.',
  inputs: [
    { id: 'image', type: 'image', label: 'Photo', required: true, limits: { max_mb: 20 }, bind: [{ node: '11', field: 'image' }] },
    {
      id: 'move',
      type: 'select',
      label: 'Camera move',
      default: 'left',
      options: [{ value: 'left', label: 'Left' }, { value: 'right', label: 'Right' }],
    },
    { id: 'details', type: 'textarea', label: 'Extra', default: '', bind: [{ node: '15', field: 'value', template: '{move} {details}' }] },
    { id: 'seed', type: 'seed', label: 'Seed', default: -1, bind: [{ node: '3:3', field: 'seed' }] },
    { id: 'unused', type: 'text', label: 'Unused' },
  ],
  models: [{ repo: 'owner/repo', file: 'split/model.safetensors', dir: 'diffusion_models', license: 'Apache-2.0' }],
  outputs: [{ node: '12', type: 'image' }],
}

const workflow = {
  '11': { class_type: 'LoadImage', inputs: { image: 'warm.png' } },
  '15': { class_type: 'PrimitiveStringMultiline', inputs: { value: 'fixed' } },
  '3:3': { class_type: 'KSampler', inputs: { seed: 1 } },
  '12': { class_type: 'SaveImage', inputs: { images: ['11', 0], filename_prefix: 'demo' } },
}

test('the manifest binds every field, including one only used inside a template', async () => {
  const manifest = await convert({
    app,
    workflow,
    id: 'freefield-demo',
    name: 'Demo',
    source: 'https://example.test/repo',
    modelInfo: async () => ({ size: 1234, sha256: 'a'.repeat(64) }),
  })
  assert.deepEqual(manifest.inputs.map((input) => input.key), ['image', 'move', 'details', 'seed'])
  assert.deepEqual(manifest.inputs.find((input) => input.key === 'move').options, ['left', 'right'])
  assert.equal(manifest.inputs.find((input) => input.key === 'seed').default, undefined, '-1 lets the host randomize')
  assert.equal(manifest.inputs.find((input) => input.key === 'image').help, 'Up to 20 MB.')
  assert.equal(manifest.workflow['11'].inputs.image, '{{image}}')
  assert.equal(manifest.workflow['15'].inputs.value, '{{move}} {{details}}')
  assert.equal(manifest.workflow['3:3'].inputs.seed, '{{seed}}')
  assert.equal(manifest.workflow['12'].inputs.images[0], '11', 'the original graph is not otherwise changed')
  assert.equal(manifest.models[0].file, 'model.safetensors')
  assert.equal(manifest.models[0].size, 1234)
  assert.equal(manifest.models[0].sha256, 'a'.repeat(64))
  assert.equal(manifest.models[0].url, 'https://huggingface.co/owner/repo/resolve/main/split/model.safetensors')
  assert.equal(manifest.output.type, 'image')
  assert.equal(manifest.estimate.vram_gb, 2)
  assert.ok(manifest.tags.includes('higgsfield') && manifest.tags.includes('freefield'))
})

test('the source graph is left untouched', async () => {
  await convert({ app, workflow, id: 'x', name: 'X', modelInfo: async () => ({}) })
  assert.equal(workflow['11'].inputs.image, 'warm.png')
  assert.equal(workflow['15'].inputs.value, 'fixed')
})
