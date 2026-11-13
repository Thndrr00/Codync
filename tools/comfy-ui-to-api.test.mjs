import assert from 'node:assert/strict'
import test from 'node:test'
import { flatten, uiToApi } from './comfy-ui-to-api.mjs'

test('a wired socket becomes a link and an unwired widget keeps its value', () => {
  const api = uiToApi({
    nodes: [
      { id: 1, type: 'LoadImage', widgets_values_named: { image: 'start.png' }, inputs: [] },
      {
        id: 2,
        type: 'KSampler',
        widgets_values_named: { seed: 7 },
        inputs: [{ name: 'model', type: 'MODEL', link: 1 }, { name: 'seed', type: 'INT', widget: { name: 'seed' }, link: null }],
      },
    ],
    links: [[1, 1, 0, 2, 0, 'MODEL']],
  })
  assert.deepEqual(api['1'], { class_type: 'LoadImage', inputs: { image: 'start.png' } })
  assert.deepEqual(api['2'].inputs, { model: [1, 0], seed: 7 })
})

test('a widget turned into a socket takes the wire and not its old value', () => {
  const api = uiToApi({
    nodes: [
      { id: 1, type: 'Seed', inputs: [] },
      {
        id: 2,
        type: 'KSampler',
        widgets_values_named: { seed: 7, steps: 20 },
        inputs: [{ name: 'seed', type: 'INT', widget: { name: 'seed' }, link: 1 }],
      },
    ],
    links: [[1, 1, 0, 2, 0, 'INT']],
  })
  assert.deepEqual(api['2'].inputs, { seed: [1, 0], steps: 20 })
})

test('a reroute is followed to the node behind it', () => {
  const api = uiToApi({
    nodes: [
      { id: 1, type: 'LoadImage', inputs: [], widgets_values_named: { image: 'a.png' } },
      { id: 2, type: 'Reroute', inputs: [{ name: '', type: 'IMAGE', link: 1 }] },
      { id: 3, type: 'PreviewImage', inputs: [{ name: 'images', type: 'IMAGE', link: 2 }] },
    ],
    links: [
      [1, 1, 0, 2, 0, 'IMAGE'],
      [2, 2, 0, 3, 0, 'IMAGE'],
    ],
  })
  assert.deepEqual(api['3'].inputs.images, [1, 0])
  assert.equal(api['2'], undefined)
})

test('a primitive node is folded into the literal it feeds', () => {
  const api = uiToApi({
    nodes: [
      { id: 1, type: 'PrimitiveNode', widgets_values: ['a prompt'] },
      { id: 2, type: 'CLIPTextEncode', widgets_values_named: { text: '' }, inputs: [{ name: 'text', type: 'STRING', widget: { name: 'text' }, link: 1 }] },
    ],
    links: [[1, 1, 0, 2, 0, 'STRING']],
  })
  assert.deepEqual(api['2'].inputs, { text: 'a prompt' })
  assert.equal(api['1'], undefined)
})

test('a muted node is dropped and a bypassed one is bridged over', () => {
  const api = uiToApi({
    nodes: [
      { id: 1, type: 'LoadImage', inputs: [], widgets_values_named: { image: 'a.png' } },
      {
        id: 2,
        type: 'LoraLoader',
        mode: 4,
        inputs: [
          { name: 'model', type: 'MODEL', link: 1 },
          { name: 'clip', type: 'CLIP', link: 2 },
        ],
        outputs: [
          { name: 'MODEL', type: 'MODEL', links: [3] },
          { name: 'CLIP', type: 'CLIP', links: [4] },
        ],
      },
      { id: 3, type: 'Muted', mode: 2, inputs: [] },
      { id: 4, type: 'KSampler', inputs: [{ name: 'model', type: 'MODEL', link: 3 }, { name: 'clip', type: 'CLIP', link: 4 }] },
    ],
    links: [
      [1, 1, 0, 2, 0, 'MODEL'],
      [2, 3, 0, 2, 1, 'CLIP'],
      [3, 2, 0, 4, 0, 'MODEL'],
      [4, 2, 1, 4, 1, 'CLIP'],
    ],
  })
  assert.deepEqual(api['4'].inputs, { model: [1, 0] })
  assert.equal(api['2'], undefined)
  assert.equal(api['3'], undefined)
})

test('a note node is dropped', () => {
  const api = uiToApi({
    nodes: [
      { id: 1, type: 'MarkdownNote', widgets_values: ['# hello'] },
      { id: 2, type: 'SaveImage', inputs: [] },
    ],
    links: [],
  })
  assert.deepEqual(Object.keys(api), ['2'])
})

test('a node type with widgets and no name table is an error', () => {
  assert.throws(
    () =>
      uiToApi({
        nodes: [{ id: 1, type: 'Mystery', widgets_values: [1], inputs: [{ name: 'amount', type: 'INT', widget: { name: 'amount' } }] }],
        links: [],
      }),
    /name table/,
  )
})

test('a subgraph is inlined with prefixed ids and its promoted wires joined', () => {
  const graph = {
    nodes: [
      { id: 1, type: 'PrimitiveNode', widgets_values: ['a.png'] },
      { id: 5, type: 'sub', inputs: [{ name: 'image', type: 'IMAGE', link: 1 }], outputs: [{ name: 'IMAGE', type: 'IMAGE', links: [2] }] },
      { id: 9, type: 'PreviewImage', inputs: [{ name: 'images', type: 'IMAGE', link: 2 }] },
    ],
    links: [
      [1, 1, 0, 5, 0, 'IMAGE'],
      [2, 5, 0, 9, 0, 'IMAGE'],
    ],
    definitions: {
      subgraphs: [
        {
          id: 'sub',
          nodes: [
            { id: -10, type: '-10', inputs: [], outputs: [{ name: 'image', type: 'IMAGE', links: [10] }] },
            { id: 11, type: 'LoadImage', inputs: [{ name: 'image', type: 'IMAGE', link: 10 }], widgets_values_named: { image: 'x.png' } },
            { id: 12, type: 'SaveImage', inputs: [{ name: 'images', type: 'IMAGE', link: 11 }] },
            { id: -20, type: '-20', inputs: [{ name: '', type: 'IMAGE', link: 12 }], outputs: [] },
          ],
          links: [
            [10, -10, 0, 11, 0, 'IMAGE'],
            [11, 12, 0, -20, 0, 'IMAGE'],
          ],
        },
      ],
    },
  }
  const flat = flatten(graph)
  assert.deepEqual(
    flat.nodes.map((node) => node.id),
    [1, '5:11', '5:12', 9],
  )
  const api = uiToApi(graph)
  assert.deepEqual(api['5:11'].inputs, { image: 'a.png' })
  assert.deepEqual(api['9'].inputs.images, ['5:12', 0])
})
