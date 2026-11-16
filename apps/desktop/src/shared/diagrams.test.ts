import assert from 'node:assert/strict'
import { test } from 'node:test'
import {
  addElement, emptyScene, newElement, parseScene, removeElement, sceneBounds, sceneSummary, serializeScene, updateElement,
} from './diagrams.ts'

test('a diagram scene round-trips through the stored JSON shape', () => {
  const scene = addElement(emptyScene(), newElement('rectangle', 20, 30))
  const reopened = parseScene(serializeScene(scene))
  assert.equal(reopened.type, 'excalidraw')
  assert.equal(reopened.elements.length, 1)
  assert.equal(reopened.elements[0]?.type, 'rectangle')
  assert.equal(reopened.elements[0]?.x, 20)
})

test('a stored scene that is not Excalidraw is refused', () => {
  assert.throws(() => parseScene(JSON.stringify({ type: 'html', elements: [] })), /Excalidraw/)
  assert.throws(() => parseScene(JSON.stringify({ type: 'excalidraw' })), /Excalidraw/)
})

test('elements move, change and disappear without changing the old scene', () => {
  const scene = addElement(emptyScene(), newElement('text', 10, 10, 'Start'))
  const id = scene.elements[0]!.id
  const moved = updateElement(scene, id, { x: 120, text: 'Finish' })
  assert.equal(scene.elements[0]?.x, 10, 'the old scene is untouched')
  assert.equal(moved.elements[0]?.x, 120)
  assert.equal(moved.elements[0]?.text, 'Finish')
  assert.equal(removeElement(moved, id).elements.length, 0)
})

test('bounds and summary describe what the person will see', () => {
  const scene = addElement(emptyScene(), newElement('ellipse', 100, 50))
  const bounds = sceneBounds(scene)
  assert.ok(bounds.x < 100 && bounds.y < 50 && bounds.width > 160 && bounds.height > 90)
  assert.match(sceneSummary(scene), /^1 element: ellipse$/)
  assert.equal(sceneSummary(emptyScene()), 'No diagram elements yet.')
})
