import assert from 'node:assert/strict'
import { test } from 'node:test'
import {
  NODE_WIDTH, botKey, connectProblem, depths, edgePath, fitView, flowEdges, flowRows, layoutNodes, mediaKey, mediaTitle, nodeAt,
  portCarry, routineKey, stepNameOf, stepProgress, zoomAt, type FlowCanvas, type FlowLink, type FlowMedia, type FlowNode, type FlowStep,
} from './routine-flow.ts'

const link = (from: string, to: string, extra: Partial<FlowLink> = {}): FlowLink =>
  ({ id: `${from}-${to}`, routineId: 'r', fromBotId: from, toBotId: to, instruction: '', createdAt: 1, ...extra })

const step = (botId: string, status: FlowStep['status'], updatedAt = 1): FlowStep =>
  ({ id: `s-${botId}`, runId: 'run', botId, deliveryId: null, input: '', output: null, status, error: null, createdAt: 1, updatedAt })

function canvas(links: FlowLink[], extra: Partial<FlowCanvas> = {}, steps: FlowStep[] = []): FlowCanvas {
  const routine = { id: 'r', botId: 'a', name: 'Digest', instruction: '', triggers: [], enabled: true, createdAt: 0, updatedAt: 0, triggerDescriptions: [] }
  return { botId: 'a', rev: 1, routines: [{ routine, recentRuns: [], upcomingRuns: [], steps }], links, positions: [], placedBotIds: [], mediaNodes: [], ...extra }
}

test('depth is the longest path from the routine bot, and a bot not reached is absent', () => {
  const d = depths('a', [link('a', 'b'), link('b', 'c'), link('a', 'c'), link('x', 'y')])
  assert.deepEqual([d.get('a'), d.get('b'), d.get('c'), d.has('x')], [0, 1, 2, false])
})

test('the connect rules are the host\'s', () => {
  const c = canvas([link('a', 'b'), link('b', 'c')], { placedBotIds: ['d'] })
  assert.match(connectProblem(c, 'r', 'b', 'b') ?? '', /itself/)
  assert.match(connectProblem(c, 'r', 'c', 'a') ?? '', /own bot starts/)
  assert.match(connectProblem(c, 'r', 'x', 'd') ?? '', /doesn't reach/)
  assert.match(connectProblem(c, 'r', 'a', 'b') ?? '', /already connected/)
  assert.match(connectProblem(c, 'r', 'c', 'b') ?? '', /loop/)
  assert.equal(connectProblem(c, 'r', 'b', 'd'), null)
  assert.equal(connectProblem(c, 'r', 'a', 'c'), null, 'a second path to a bot is a join')
  assert.match(connectProblem(c, 'nope', 'a', 'b') ?? '', /not on this canvas/)
})

test('the depth limit counts what already hangs below the bot being moved', () => {
  const c = canvas([link('a', 'b'), link('a', 'x'), link('x', 'y'), link('y', 'z')])
  assert.match(connectProblem(c, 'r', 'b', 'x', 3) ?? '', /at most 3/)
  assert.equal(connectProblem(c, 'r', 'b', 'x', 4), null)
})

test('nodes never moved sit in columns by depth, and a saved position wins', () => {
  const c = canvas([link('a', 'b'), link('b', 'c')], { placedBotIds: ['z'], positions: [{ nodeKey: botKey('b'), x: 5, y: 7 }] })
  const nodes = new Map(layoutNodes(c).map((n) => [n.key, n]))
  const colX = (key: string) => nodes.get(key)!.x
  assert.ok(colX(routineKey('r')) < colX(botKey('a')) && colX(botKey('a')) < colX(botKey('c')))
  assert.deepEqual([nodes.get(botKey('b'))!.x, nodes.get(botKey('b'))!.y, nodes.get(botKey('b'))!.saved], [5, 7, true])
  assert.ok(colX(botKey('z')) > colX(botKey('c')), 'a bot no flow reaches stands after the flows')
  const fan = layoutNodes(canvas([link('a', 'b'), link('a', 'c')]))
  const [nb, nc] = [botKey('b'), botKey('c')].map((key) => fan.find((n) => n.key === key)!)
  assert.ok(nb.x === nc.x && nb.y !== nc.y, 'nodes of one column don\'t overlap')
})

test('edges: each routine to its bot, then one per link', () => {
  const edges = flowEdges(canvas([link('a', 'b')]))
  assert.deepEqual(edges.map((e) => [e.from, e.to, e.linkId]), [[routineKey('r'), botKey('a'), null], [botKey('a'), botKey('b'), 'a-b']])
})

test('the list mirrors the canvas depth first with the latest step of each bot', () => {
  const c = canvas([link('a', 'b'), link('b', 'c'), link('a', 'd')], {}, [step('a', 'succeeded'), step('b', 'failed'), step('c', 'skipped')])
  const rows = flowRows(c)
  assert.deepEqual(rows.map((r) => [r.nodeId, r.depth, r.step?.status ?? null]), [['a', 0, 'succeeded'], ['b', 1, 'failed'], ['c', 2, 'skipped'], ['d', 1, null]])
})

test('a loop in the links can\'t run the list away', () => {
  const rows = flowRows(canvas([link('a', 'b'), link('b', 'a')]))
  assert.deepEqual(rows.map((r) => r.nodeId), ['a', 'b'])
})

test('a step name comes from its link or media node, and empty falls back to the type', () => {
  const media: FlowMedia = {
    id: 'm1', name: 'Hero shot', kind: 'image', model: 'gpt-image', prompt: '', aspect: '1:1',
    duration: 6, resolution: '480p', transparent: false, voice: null, createdAt: 1, routineId: 'r',
  }
  const c = canvas([link('a', 'b', { name: 'Translate' })], { mediaNodes: [media] })
  assert.equal(stepNameOf(c, botKey('b')), 'Translate', 'a bot step is named by the link that reaches it')
  assert.equal(stepNameOf(c, mediaKey('m1')), 'Hero shot')
  assert.equal(stepNameOf(c, botKey('c')), '', 'an unnamed step falls back to its type in the client')
})

test('a running step shows its predecessor and how many steps are done', () => {
  const steps = [step('a', 'succeeded'), step('b', 'running')]
  const c = canvas([link('a', 'b')], {}, steps)
  const progress = stepProgress(c.routines[0], steps[1])
  assert.equal(progress.previous?.botId, 'a')
  assert.equal(progress.done, 1)
  assert.equal(progress.total, 2)
  const node = (kind: FlowNode['kind']): FlowNode => ({ key: `${kind}:x`, kind, id: 'x', x: 0, y: 0, saved: false })
  assert.equal(portCarry(node('bot'), null), 'text')
  assert.equal(portCarry(node('media'), { kind: 'image' } as FlowMedia), 'image')
  assert.equal(portCarry(node('media'), { kind: 'video' } as FlowMedia), 'file')
})

test('zoom keeps the point under the pointer still and stays within its bounds', () => {
  const view = zoomAt({ x: 10, y: 20, scale: 1 }, 2, 100, 100)
  assert.deepEqual([view.scale, view.x, view.y], [2, -80, -60])
  assert.equal(zoomAt({ x: 0, y: 0, scale: 1 }, 100, 0, 0).scale, 2)
  assert.equal(zoomAt({ x: 0, y: 0, scale: 1 }, 0.001, 0, 0).scale, 0.3)
})

test('fit shows every node, centered', () => {
  const nodes = layoutNodes(canvas([link('a', 'b')]))
  const view = fitView(nodes, 1000, 600)
  for (const n of nodes) {
    assert.ok(n.x * view.scale + view.x >= 0 && (n.x + NODE_WIDTH) * view.scale + view.x <= 1000)
  }
  assert.deepEqual(fitView([], 1000, 600), { x: 0, y: 0, scale: 1 })
})

test('edges leave the right side and a drop finds the card under it', () => {
  const [from, to] = layoutNodes(canvas([link('a', 'b')])).filter((n) => n.kind === 'bot')
  assert.match(edgePath(from, to), /^M \S+ \S+ C /)
  assert.equal(nodeAt([from, to], from.x + 5, from.y + 5)?.key, from.key)
  assert.equal(nodeAt([from, to], -999, -999), null)
})

const media = (id: string, kind: FlowMedia['kind'] = 'image', extra: Partial<FlowMedia> = {}): FlowMedia =>
  ({ id, kind, model: 'gpt-image', prompt: '', aspect: '1:1', duration: 4, resolution: '720p', transparent: false, voice: null, createdAt: 1, routineId: 'r', ...extra })

test('a media step is a node of its own, one column after the step that reaches it', () => {
  const c = canvas([link('a', `media:m1`), link('a', 'b')], { mediaNodes: [media('m1'), media('m2', 'video', { routineId: 'other' })] })
  const nodes = new Map(layoutNodes(c).map((n) => [n.key, n]))
  const bot = nodes.get(botKey('a'))!
  assert.equal(nodes.get(mediaKey('m1'))!.kind, 'media')
  assert.ok(nodes.get(mediaKey('m1'))!.x > bot.x, 'a handed-on step follows the step before it')
  assert.ok(nodes.get(mediaKey('m2'))!.x > bot.x, 'a step no flow reaches still has a place')
  assert.deepEqual(flowEdges(c).map((e) => [e.from, e.to]), [[routineKey('r'), botKey('a')], [botKey('a'), mediaKey('m1')], [botKey('a'), botKey('b')]])
})

test('a media step stands in the connect rules like a bot, but only in its own routine', () => {
  const c = canvas([link('a', 'b')], { mediaNodes: [media('m1'), media('m2'), media('m3', 'video', { routineId: 'other' })] })
  assert.equal(connectProblem(c, 'r', 'a', 'media:m1'), null)
  assert.equal(connectProblem(c, 'r', 'a', 'media:m2'), null, 'a join on a second path is fine')
  assert.match(connectProblem(c, 'r', 'a', 'media:m3') ?? '', /not part of this routine/)
  assert.match(connectProblem(c, 'nope', 'a', 'media:m1') ?? '', /not on this canvas/)
  const looped = canvas([link('a', 'media:m1'), link('media:m1', 'b')], { mediaNodes: [media('m1')], placedBotIds: ['c'] })
  assert.match(connectProblem(looped, 'r', 'b', 'media:m1') ?? '', /loop/)
  assert.match(connectProblem(looped, 'r', 'media:m1', 'b') ?? '', /already connected/)
  assert.equal(connectProblem(looped, 'r', 'media:m1', 'c'), null)
})

test('the list shows a media step under the step that hands to it', () => {
  const c = canvas([link('a', 'media:m1'), link('media:m1', 'b')], { mediaNodes: [media('m1', 'image', { prompt: 'A red fox' })] },
    [step('media:m1', 'succeeded')])
  const rows = flowRows(c)
  assert.deepEqual(rows.map((r) => [r.nodeId, r.kind, r.step?.status ?? null]), [['a', 'bot', null], ['media:m1', 'media', 'succeeded'], ['b', 'bot', null]])
  assert.equal(mediaTitle(rows[1]!.media), 'A red fox')
  assert.equal(mediaTitle(media('m9')), 'Picture')
  assert.equal(mediaTitle(media('m9', 'video')), 'Video')
})
