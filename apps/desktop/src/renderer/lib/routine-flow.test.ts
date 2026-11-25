import assert from 'node:assert/strict'
import { test } from 'node:test'
import {
  NODE_WIDTH, answerResultRows, botKey, canvasCenter, canvasRunning, chipBefore, connectProblem, depths, edgePath, fieldTokens, fieldTrigger,
  fieldsForStep, fitView, flowEdges, flowRows, folderOf, formatAnswerSpec, graphEdgeProblem, groupMediaResults, historyStatus, insertFieldToken,
  isFlowPaused, isFlowReadOnly, isAnnotationKind, nodeEditable, nodeTitle, nextRunCursor, problemsForStep, qualifiedFieldName, runSummary,
  stepMarks, strokePath, triggerFields,
  isNewerSetting, layoutNodes, matchingFields, mediaBatch, mediaKey, mediaRetryLabel, mediaRunLabel, mediaStepsFor,
  mediaTitle, mediaTotalCredits, mediaUnitPrice, missingMediaModel, nodeAt, parseAnswerSpec, parseWorkflowFile, portCarry,
  routineKey, specKeyOf, specText, statusLabel, stepNameOf, stepProgress, tidyLayout, unknownSettings, unavailableMediaModel,
  availableMediaModels, workflowFileBytes, workflowsInFolder, zoomAt,
  type FieldMatch, type FlowCanvas, type FlowField, type FlowLink, type FlowMedia, type FlowMediaCatalog, type FlowNode,
  type FlowNodeRegistry, type FlowStep, type WorkflowFoldersSnapshot, type WorkflowNode, type WorkflowRun,
} from './routine-flow.ts'
import {
  closeOtherWorkflows, closeWorkflow, closeWorkflowsRight, moveWorkflow, nextActiveTab, openWorkflow,
  restoreWorkflowTabs, safeWorkflowId,
} from '../views/routines/flow/flow-tabs.ts'

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

test('tidy puts steps in run-order columns, stacks branches, and ignores saved places', () => {
  const c = canvas([link('a', 'b'), link('a', 'c'), link('b', 'd')], {
    positions: [{ nodeKey: botKey('b'), x: 9999, y: 9999 }],
    placedBotIds: ['z'],
  })
  const at = new Map(tidyLayout(c).positions.map((p) => [p.nodeKey, p]))
  const x = (key: string) => at.get(key)!.x
  assert.ok(x(routineKey('r')) < x(botKey('a')), 'the routine card starts the first column')
  assert.ok(x(botKey('a')) < x(botKey('b')), 'the next step is one column right')
  assert.equal(x(botKey('b')), x(botKey('c')), 'branches share a column')
  assert.equal(at.get(botKey('b'))!.y < at.get(botKey('c'))!.y, true, 'branches sit below each other')
  assert.ok(x(botKey('d')) > x(botKey('b')), 'a later step is farther right')
  assert.notEqual(at.get(botKey('b'))!.x, 9999, 'a saved position does not win')
  assert.ok(x(botKey('z')) > x(botKey('d')), 'a hand-placed bot comes after the flows')
})

test('tidy moves a group as one block and brings its card along', () => {
  const c = canvas([link('a', 'b'), link('b', 'c'), link('c', 'd')])
  const layout = tidyLayout(c, [{ id: 'g1', members: [botKey('b'), botKey('c')] }])
  const at = new Map(layout.positions.map((p) => [p.nodeKey, p]))
  assert.equal(at.get(botKey('b'))!.x, at.get(botKey('c'))!.x, 'the group stays in one column')
  assert.equal(at.get(botKey('c'))!.y - at.get(botKey('b'))!.y, 160, 'its members keep their spacing')
  assert.deepEqual(layout.groups, [{ id: 'g1', x: at.get(botKey('b'))!.x, y: at.get(botKey('b'))!.y }])
  const places = layout.positions.map((p) => `${p.x}:${p.y}`)
  assert.equal(new Set(places).size, places.length, 'no two steps land on the same place')
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

test('a canvas is running when a step or a recent run is still active', () => {
  assert.equal(canvasRunning(null), false)
  assert.equal(canvasRunning(canvas([])), false)
  assert.equal(canvasRunning(canvas([], {}, [step('b', 'running')])), true)
  assert.equal(canvasRunning(canvas([], {}, [step('b', 'queued')])), true, 'a posted message keeps the tab running')
  const run = { id: 'r1', routineId: 'r', botId: 'a', status: 'recovering', createdAt: 1 }
  assert.equal(canvasRunning(canvas([], { routines: [{ ...canvas([]).routines[0], recentRuns: [run] }] })), true)
  const done = { ...run, status: 'succeeded' }
  assert.equal(canvasRunning(canvas([], { routines: [{ ...canvas([]).routines[0], recentRuns: [done] }] })), false)
})

test('the canvas centre follows pan and zoom, so paste lands where the person looks', () => {
  assert.deepEqual(canvasCenter({ x: 0, y: 0, scale: 1 }, 800, 600), { x: 400, y: 300 })
  assert.deepEqual(canvasCenter({ x: -100, y: -50, scale: 2 }, 800, 600), { x: 250, y: 175 })
  assert.deepEqual(canvasCenter({ x: 40, y: 10, scale: 0.5 }, 200, 100), { x: 120, y: 80 })
})

test('a workflow sits in one folder and the unfiled list is the rest, in order', () => {
  const folders: WorkflowFoldersSnapshot = {
    revision: 2,
    folders: [{ id: 'f1', name: 'Reports' }, { id: 'f2', name: 'Images' }],
    placement: { a: 'f1', b: 'f1', c: 'gone' },
  }
  assert.equal(folderOf(folders, 'a'), 'f1')
  assert.equal(folderOf(folders, 'c'), null, 'a placement to a deleted folder is unfiled')
  assert.equal(folderOf(folders, 'z'), null)
  assert.equal(folderOf(null, 'a'), null)
  assert.deepEqual(workflowsInFolder(folders, ['a', 'b', 'c', 'z'], 'f1'), ['a', 'b'])
  assert.deepEqual(workflowsInFolder(folders, ['a', 'b', 'c', 'z'], null), ['c', 'z'])
  assert.deepEqual(workflowsInFolder(null, ['a', 'b'], null), ['a', 'b'])
})

test('a saved tab list keeps only safe ids, in order, and brings the opened workflow back', () => {
  assert.deepEqual(restoreWorkflowTabs(['a', 'b', 'a', '', 'x/y', 4, null, 'b'], 'c'), ['a', 'b', 'c'])
  assert.deepEqual(restoreWorkflowTabs(null, 'a'), ['a'])
  assert.deepEqual(restoreWorkflowTabs('nope', 'a'), ['a'])
  assert.deepEqual(restoreWorkflowTabs(['b', 'a'], 'a'), ['b', 'a'], 'an already open workflow keeps its place')
})

test('only plain host ids are safe to save', () => {
  assert.equal(safeWorkflowId('bot_12'), true)
  assert.equal(safeWorkflowId('a/b'), false)
  assert.equal(safeWorkflowId('a b'), false)
  assert.equal(safeWorkflowId('a\nb'), false)
  assert.equal(safeWorkflowId(''), false)
  assert.equal(safeWorkflowId('x'.repeat(201)), false)
  assert.equal(safeWorkflowId(7), false)
})

test('opening moves a repeat tab to the end and keeps a fresh id out of a bad list', () => {
  assert.deepEqual(openWorkflow(['a', 'b', 'c'], 'b'), ['a', 'c', 'b'])
  assert.deepEqual(openWorkflow(['a'], 'b'), ['a', 'b'])
  assert.deepEqual(openWorkflow(['a'], 'a b'), ['a'])
})

test('closing keeps one tab and selects the right neighbour, else the left one', () => {
  assert.deepEqual(closeWorkflow(['a', 'b', 'c'], 'b'), ['a', 'c'])
  assert.deepEqual(closeWorkflow(['a', 'b'], 'a'), ['b'])
  assert.deepEqual(closeWorkflow(['only'], 'only'), ['only'], 'the last tab cannot close')
  assert.deepEqual(closeWorkflow(['a', 'b'], 'missing'), ['a', 'b'])
  assert.equal(nextActiveTab(['a', 'b', 'c'], 'a', 'a'), 'b')
  assert.equal(nextActiveTab(['a', 'b', 'c'], 'c', 'c'), 'b')
  assert.equal(nextActiveTab(['a', 'b', 'c'], 'b', 'a'), null, 'a different tab being closed leaves the selection alone')
  assert.equal(nextActiveTab(['only'], 'only', 'only'), null)
  assert.equal(nextActiveTab(['a', 'b'], 'missing', 'missing'), 'a')
})

test('the tab menu closes others or everything to the right', () => {
  assert.deepEqual(closeOtherWorkflows(['a', 'b', 'c'], 'b'), ['b'])
  assert.deepEqual(closeOtherWorkflows(['a', 'b'], 'missing'), ['a', 'b'])
  assert.deepEqual(closeWorkflowsRight(['a', 'b', 'c', 'd'], 'b'), ['a', 'b'])
  assert.deepEqual(closeWorkflowsRight(['a', 'b'], 'b'), ['a', 'b'])
  assert.deepEqual(closeWorkflowsRight(['a', 'b'], 'missing'), ['a', 'b'])
})

test('a dragged tab takes the target\'s place in the strip', () => {
  assert.deepEqual(moveWorkflow(['a', 'b', 'c', 'd'], 'd', 'b'), ['a', 'd', 'b', 'c'])
  assert.deepEqual(moveWorkflow(['a', 'b', 'c'], 'b', 'a'), ['b', 'a', 'c'])
  assert.deepEqual(moveWorkflow(['a', 'b'], 'a', 'a'), ['a', 'b'])
  assert.deepEqual(moveWorkflow(['a', 'b'], 'a', 'missing'), ['a', 'b'])
})

// MARK: typed step fields

const field = (name: string, step = '', type: FlowField['type'] = 'text', values: string[] = []): FlowField =>
  ({ name, step, type, values, icon: step ? 'bell' : 'clock' })

test('chips are only whole {{names}} and the raw text survives a round trip', () => {
  const text = 'Use {{count}} then {{priority}}; keep {{bad name}} and {{open'
  assert.deepEqual(fieldTokens(text).map((token) => token.kind === 'field' ? token.name : token.text),
    ['Use ', 'count', ' then ', 'priority', '; keep {{', 'bad name}} and {{open'])
  assert.equal(fieldTokens(text).map((token) => token.kind === 'field' ? token.raw : token.text).join(''), text)
  assert.deepEqual(fieldTokens('plain text'), [{ kind: 'text', text: 'plain text' }])
  assert.deepEqual(fieldTokens(''), [])
})

test('Backspace takes a whole chip only at its closing braces', () => {
  const text = 'Sum {{count}} now'
  assert.deepEqual(chipBefore(text, 13), { start: 4, end: 13, name: 'count' })
  assert.equal(chipBefore(text, 12), null, 'inside the chip is an ordinary character')
  assert.equal(chipBefore('{{bad name}}', 12), null)
  assert.equal(chipBefore('no chip', 7), null)
})

test('{{ and a single { both complete to {{name}}, replacing the typed letters', () => {
  assert.deepEqual(insertFieldToken('Sum {{cou', 9, 'count'), { text: 'Sum {{count}}', caret: 13 })
  assert.deepEqual(insertFieldToken('Sum {', 5, 'count'), { text: 'Sum {{count}}', caret: 13 })
  assert.deepEqual(insertFieldToken('Sum ', 4, 'count'), { text: 'Sum {{count}}', caret: 13 })
  const after = insertFieldToken('Sum {{count}}', 13, 'later')
  assert.equal(after.text, 'Sum {{count}}{{later}}')
})

test('a completion trigger ignores closed chips, JSON and braces stuck to a word', () => {
  assert.deepEqual(fieldTrigger('{{pri', 5), { start: 0, query: 'pri', single: false })
  assert.deepEqual(fieldTrigger('Say {', 5), { start: 4, query: '', single: true })
  assert.equal(fieldTrigger('{{count}}', 9), null)
  assert.equal(fieldTrigger('{"a": ', 6), null)
  assert.equal(fieldTrigger('x{', 2), null)
})

test('field filtering puts prefix matches first and bolds the typed letters', () => {
  const fields = [field('priority', 'LLM', 'choice', ['high', 'low']), field('note'), field('printer'), field('count')]
  const matches: FieldMatch[] = matchingFields(fields, 'PRI')
  assert.deepEqual(matches.map((m) => m.field.name), ['priority', 'printer'])
  assert.deepEqual(matches[0].parts, [
    { text: 'pri', match: true }, { text: 'ority', match: false },
  ])
  assert.deepEqual(matchingFields(fields, 'zzz'), [], 'the list shows "No matches"')
  assert.deepEqual(matchingFields(fields, '').map((m) => m.field.name), ['priority', 'note', 'printer', 'count'])
})

test('a step offers the trigger\'s fields and earlier steps\' fields, nearest first', () => {
  const c = canvas([link('a', 'b'), link('b', 'c')], {
    triggerFields: [field('when')],
    flowFields: { a: [field('count', 'A')], b: [field('count', 'B'), field('priority', 'B')], c: [field('later', 'C')] },
  })
  assert.deepEqual(fieldsForStep(c, botKey('b')).map((f) => f.name), ['when', 'count'])
  assert.deepEqual(fieldsForStep(c, botKey('c')).map((f) => [f.name, f.step]), [['when', ''], ['count', 'B'], ['priority', 'B']])
  assert.equal(fieldsForStep(c, 'not-a-step').length, 4, 'an unplaced step sees all fields')
  assert.equal(isFlowPaused({ ...c, pausedRoutineIds: ['r'] }, 'r'), true)
  assert.equal(isFlowPaused(c, 'r'), false)
})

test('answer rows keep the host\'s name: type text, with unshown lines kept apart', () => {
  const parsed = parseAnswerSpec('count: int; priority: one of high, low; note?: text; free form')
  assert.deepEqual(parsed.rows, [
    { name: 'count', type: 'integer', values: [], optional: false },
    { name: 'priority', type: 'choice', values: ['high', 'low'], optional: false },
    { name: 'note', type: 'text', values: [], optional: true },
  ])
  assert.equal(parsed.unknown, 'free form')
  assert.equal(formatAnswerSpec(parsed.rows), 'count: int\npriority: one of high, low\nnote?: text')
  assert.deepEqual(parseAnswerSpec('priority: one of high').rows, [], 'a choice needs two values')
})

test('a queued step names the waiting state instead of counting as done', () => {
  assert.equal(statusLabel('queued'), 'Queued')
  assert.equal(statusLabel(null), 'Not run yet')
  const c = canvas([], {}, [step('a', 'succeeded'), step('b', 'queued')])
  assert.deepEqual(stepProgress(c.routines[0], c.routines[0].steps[1]!), { previous: c.routines[0].steps[0], done: 1, total: 2 })
})

test('an answer-field result shows one row per field with real line breaks', () => {
  assert.deepEqual(answerResultRows('{"summary":"Line one\\nLine two","count":3,"urgent":true,"tags":["a","b"]}'), [
    { name: 'summary', value: 'Line one\nLine two' },
    { name: 'count', value: '3' },
    { name: 'urgent', value: 'yes' },
    { name: 'tags', value: 'a, b' },
  ])
  // A literal backslash-n in the value stays text; it is never rendered as a line break.
  const literal = answerResultRows('{"note":"C:\\\\temp\\\\n"}')[0]!
  assert.equal(literal.value, 'C:\\temp\\n')
  assert.equal(literal.value.includes('\n'), false)
  assert.deepEqual(answerResultRows('a plain answer'), [], 'a text answer has no field rows')
  assert.deepEqual(answerResultRows(null), [])
  assert.deepEqual(answerResultRows('[1,2]'), [], 'an array is not an answer object')
})

const registry: FlowNodeRegistry = {
  bot: { effort: { values: ['low', 'medium', 'high'] }, filters: { values: ['a', 'b'], list: true }, task: {} },
  image: { model: {}, prompt: {} },
  video: { model: {}, prompt: {} },
}

test('value validation refuses an unknown value with the allowed ones', () => {
  assert.deepEqual(unknownSettings('bot', { effort: 'extreme' }, registry),
    [{ name: 'effort', values: ['low', 'medium', 'high'], newer: false }])
  assert.deepEqual(unknownSettings('bot', { effort: 'high' }, registry), [])
  assert.deepEqual(unknownSettings('bot', { filters: 'a, z' }, registry),
    [{ name: 'filters', values: ['a', 'b'], newer: false }])
  assert.deepEqual(unknownSettings('bot', { filters: ['a', 'b'] }, registry), [])
  assert.deepEqual(unknownSettings('bot', { odd: 'x' }, registry), [{ name: 'odd', values: [], newer: true }])
  assert.deepEqual(unknownSettings('bot', { effort: 'extreme' }, null), [], 'an older host has no registry')
  assert.equal(isNewerSetting('bot', { odd: 'x' }, registry), true)
  assert.equal(isNewerSetting('bot', { effort: 'extreme' }, registry), false)
})

test('a workflow with a newer setting is read-only, an unknown value is not', () => {
  const c = canvas([])
  assert.equal(isFlowReadOnly(c, registry), false)
  const newer = canvas([], { links: [link('a', 'b', { modelOptions: { Odd: 'x' } })] })
  assert.equal(isFlowReadOnly(newer, registry), true)
  const refused = canvas([], { links: [link('a', 'b', { modelOptions: { effort: 'extreme' } })] })
  assert.equal(isFlowReadOnly(refused, registry), false)
  assert.equal(isFlowReadOnly({ ...c, readOnly: true }, registry), true)
  assert.equal(isFlowReadOnly({ ...c, formatVersion: 2 }, null), true, 'a newer document is read-only without a registry')
})

test('host specs and step problems come in either shape', () => {
  assert.equal(specKeyOf('bot:b'), 'b')
  assert.equal(specKeyOf('media:m1'), 'media:m1')
  assert.equal(specText({ answer: 'count: int', check: 'count > 2' }, 'answer'), 'count: int')
  assert.equal(specText({ answer: 'count: int', check: 'count > 2' }, 'check'), 'count > 2')
  assert.equal(specText('count: int', 'answer'), 'count: int')
  assert.equal(specText(undefined, 'check'), '')
  const c = canvas([], { answerProblems: [{ nodeKey: 'media:m1', text: 'Not every path gives count.' }] })
  assert.equal(problemsForStep(c, 'media:m1')['*'], 'Not every path gives count.')
  assert.deepEqual(problemsForStep(c, botKey('a')), {})
})

test('a pause stamped on the routine card shows on the canvas', () => {
  const c = canvas([])
  c.routines[0].paused = true
  assert.equal(isFlowPaused(c, 'r'), true)
})

test('only batches of one to four count; anything else makes one result', () => {
  for (const size of [1, 2, 3, 4]) assert.equal(mediaBatch(size), size)
  for (const bad of [0, 5, -1, 1.5, '2', null, undefined, Number.NaN]) assert.equal(mediaBatch(bad), 1)
})

test('the Run button shows one price times the batch and the number of results', () => {
  const tr = (text: string) => text
  assert.equal(mediaTotalCredits(12, 4), 48)
  assert.equal(mediaTotalCredits(12, 9), 12, 'an invalid batch costs one result')
  assert.equal(mediaTotalCredits(null, 4), null, 'no price means no price is shown')
  assert.equal(mediaRunLabel(12, 3, false, tr), 'Run · 36 credits · 3 results')
  assert.equal(mediaRunLabel(12, 1, false, tr), 'Run · 12 credits')
  assert.equal(mediaRunLabel(null, 4, false, tr), 'Run · 4 results')
  assert.equal(mediaRunLabel(12, 4, true, tr), 'Model not available')
  assert.equal(mediaRetryLabel(12, tr), 'Retry · 12 credits')
  assert.equal(mediaRetryLabel(null, tr), 'Retry')
})

test('the model picker follows the host availability flag but keeps a saved model', () => {
  const base = {
    id: 'gpt-image', name: 'GPT Image', maker: 'OpenAI', subscription: 'ChatGPT', kinds: ['image' as const],
    aspects: ['1:1'], transparent: true, voices: false,
    capabilities: { aspects: ['1:1'], durations: [], resolutions: [], transparent: true, voices: null },
  }
  const answer: FlowMediaCatalog = {
    agent: 'codex', installed: true,
    models: [
      { ...base, available: true, batchCredits: { credits: 7 } },
      { ...base, id: 'gone', name: 'Old Model', available: false, batchCredits: { credits: null } },
    ],
    modelsReady: true,
    unavailableModels: [{ id: 'gone', name: 'Old Model', kinds: ['image'] }],
  }
  assert.deepEqual(availableMediaModels(answer, 'image').map((model) => model.id), ['gpt-image'])
  assert.equal(unavailableMediaModel(answer, 'gone', 'image')?.name, 'Old Model')
  assert.equal(missingMediaModel(answer, 'gone', 'image'), true)
  assert.equal(missingMediaModel(answer, 'gpt-image', 'image'), false)
  assert.equal(mediaUnitPrice(answer, 'gpt-image'), 7)
  assert.equal(mediaUnitPrice(answer, 'gone'), null, 'a subscription has no credit price')
  assert.equal(mediaUnitPrice({ ...answer, price: 9 }, 'gpt-image'), 9)
  assert.equal(mediaUnitPrice(null, 'gpt-image'), null)
  // An older answer has no flags or prices: every model stays listed and none is marked missing.
  const older: FlowMediaCatalog = {
    agent: 'codex', installed: true,
    models: [base, { ...base, id: 'legacy', name: 'Legacy' }],
  }
  assert.deepEqual(availableMediaModels(older, 'image').map((model) => model.id), ['gpt-image', 'legacy'])
  assert.equal(missingMediaModel(older, 'legacy', 'image'), false)
  assert.equal(unavailableMediaModel(older, 'legacy', 'image'), null)
  assert.equal(unavailableMediaModel(older, 'removed', 'image')?.name, 'removed', 'a model gone from a loaded list is marked')
  // A model flagged unavailable is marked even before a "list loaded" flag exists.
  assert.equal(missingMediaModel({ ...older, models: [{ ...base, available: false }] }, 'gpt-image', 'image'), true)
})

test('a batch groups into pending, failed and succeeded results with one file walk', () => {
  const file = (id: string) => ({ id, name: `${id}.png`, size: 1, sha256: '' })
  const batched: FlowStep = {
    ...step('media:run1', 'running'),
    files: [file('c1'), file('d1')],
    results: [
      { index: 0, status: 'succeeded' },
      { index: 1, status: 'failed', error: 'The model of this step is not available' },
      { index: 2, status: 'succeeded' },
      { index: 3, status: 'pending' },
    ],
  }
  const grouped = groupMediaResults([batched], 5)
  assert.deepEqual(grouped.pending.map((item) => [item.id, item.status]), [['s-media:run1:3', 'pending']])
  assert.deepEqual(grouped.failed.map((item) => [item.error, item.credits]), [
    ['The model of this step is not available', 5],
  ])
  assert.deepEqual(grouped.succeeded.map((item) => item.id), ['s-media:run1:0', 's-media:run1:2'])
  assert.deepEqual(grouped.files.map((item) => item.id), ['c1', 'd1'], 'a failure does not shift the files')
  assert.deepEqual(groupMediaResults([step('media:a', 'running')], 5).pending.map((t) => t.status), ['pending'])
  const blocked = {
    ...step('media:run2', 'failed'), error: 'The model of this step is not available',
    results: [{ index: 0, status: 'pending' as const }, { index: 1, status: 'pending' as const }],
  }
  const stopped = groupMediaResults([blocked], 5)
  assert.equal(stopped.failed.length, 1, 'a run that stops before making anything shows one failure')
  assert.equal(stopped.failed[0].error, 'The model of this step is not available')
  assert.equal(stopped.pending.length, 0)
  const big = { ...batched, results: Array.from({ length: 6 }, (_, i) => ({ index: i, status: 'succeeded' as const })) }
  assert.ok(groupMediaResults([big], 5).files.length <= 4, 'only the newest batch is shown')
})

test('an older host with one step per result still groups by the step state', () => {
  const file = { id: 'f1', name: 'f1.png', size: 1, sha256: '' }
  const grouped = groupMediaResults([step('media:m1', 'succeeded', 1)].map((s) => ({ ...s, files: [file] })), null)
  assert.equal(grouped.succeeded.length, 1)
  assert.deepEqual(grouped.files, [file])
  assert.deepEqual(groupMediaResults([step('media:m1', 'running')], null).pending.map((t) => t.status), ['pending'])
  assert.equal(groupMediaResults([step('media:m1', 'skipped')], null).succeeded.length, 0, 'a skipped step has no result')
  const c = canvas([], {}, [
    { ...step('media:old', 'succeeded', 1), updatedAt: 1 },
    { ...step('media:m1', 'running', 2), updatedAt: 2 },
  ])
  assert.deepEqual(mediaStepsFor(c, 'media:m1').map((s) => s.updatedAt), [2])
  const live = canvas([], { stepSettings: {
    's-media:m1': {
      kind: 'image', inputs: [],
      results: [{ index: 0, status: 'pending' }, { index: 1, status: 'failed', error: 'no file' }],
    },
  } }, [step('media:m1', 'running', 2)])
  const merged = groupMediaResults(mediaStepsFor(live, 'media:m1'), null)
  assert.deepEqual(merged.pending.map((take) => take.index), [0], 'batch progress comes from step settings')
  assert.equal(merged.failed[0].error, 'no file')
})

test('a portable workflow file gives its names in plain or base64 form', () => {
  const file = {
    formatVersion: 1,
    workflow: { name: 'Digest', agents: ['Ada'], channels: ['#news'], steps: [] },
    assets: { 'a.png': '...', 'b.png': '...' },
  }
  const json = JSON.stringify(file)
  const base64 = Buffer.from(json, 'utf8').toString('base64')
  for (const raw of [json, base64]) {
    assert.deepEqual(parseWorkflowFile(raw), { name: 'Digest', agents: ['Ada'], channels: ['#news'], assets: 2 })
  }
  assert.equal(new TextDecoder().decode(workflowFileBytes(json)), json)
  assert.equal(new TextDecoder().decode(workflowFileBytes(base64)), json)
  assert.equal(parseWorkflowFile('not a file'), null)
  assert.equal(parseWorkflowFile('{}'), null, 'a file without steps is not a workflow')
})

// MARK: portable workflow graph

test('a trigger offers the same typed fields the host freezes on a run', () => {
  const gh = triggerFields('github')
  assert.equal(gh.length, 24)
  assert.deepEqual(gh.slice(0, 6).map((field) => field.name), ['repository', 'actor', 'action', 'number', 'pull_request', 'url'])
  assert.deepEqual(gh.slice(-3).map((field) => field.name), ['check_conclusion', 'check_app', 'head_branch'])
  assert.equal(gh.find((field) => field.name === 'number')?.type, 'integer')
  assert.equal(gh.find((field) => field.name === 'pull_request')?.type, 'yesNo')
  assert.equal(gh.find((field) => field.name === 'labels')?.type, 'list')
  assert.deepEqual(triggerFields('schedule').map((field) => field.name), ['run_time', 'local_date', 'local_time', 'weekday'])
  assert.deepEqual(triggerFields('discord').map((field) => field.name),
    ['text', 'author', 'author_id', 'message_id', 'channel', 'channel_id', 'server'])
  assert.equal(triggerFields('telegram').length, 6)
  assert.equal(triggerFields('slack').length, 8)
  assert.deepEqual(triggerFields('manual'), [])
})

test('a line follows the ports and a condition needs its yes or no branch', () => {
  assert.equal(graphEdgeProblem('llm', 'condition', 'text'), null, 'a plain line has no branch')
  assert.equal(graphEdgeProblem('llm', 'condition', 'text', 'yes'), 'branch', 'only a condition may branch')
  assert.equal(graphEdgeProblem('condition', 'approval', 'text', 'no'), null)
  assert.equal(graphEdgeProblem('condition', 'approval', 'text'), 'condition-branch')
  assert.equal(graphEdgeProblem('llm', 'approval', 'text', 'yes'), 'branch')
  assert.equal(graphEdgeProblem('asset', 'llm', 'image'), null, 'an asset feeds any port')
  assert.equal(graphEdgeProblem('image', 'output', 'file'), null)
  assert.equal(graphEdgeProblem('image', 'condition', 'image'), 'port')
  assert.equal(graphEdgeProblem('note', 'llm', 'text'), 'annotation')
  assert.equal(graphEdgeProblem('llm', 'note', 'text'), 'annotation')
  assert.equal(graphEdgeProblem('nope' as 'llm', 'llm', 'text'), 'port')
  assert.equal(isAnnotationKind('shape'), true)
  assert.equal(isAnnotationKind('agent'), false)
})

test('a locked or unknown node is read-only on its own, not the whole workflow', () => {
  const node = (kind: string, locked = false): WorkflowNode =>
    ({ id: 'n', kind: kind as WorkflowNode['kind'], at: { x: 0, y: 0 }, locked, fields: {} })
  assert.equal(nodeEditable(node('llm')), true)
  assert.equal(nodeEditable(node('llm', true)), false)
  assert.equal(nodeEditable(node('future')), false)
  assert.equal(nodeEditable(node('llm'), true), false, 'a newer-app workflow locks every node')
  assert.notEqual(nodeTitle(node('llm')), '')
  assert.equal(nodeTitle({ ...node('condition'), fields: { question: 'Is it urgent?' } }), 'Is it urgent?')
  assert.equal(nodeTitle(node('approval')), 'Approval')
})

test('a freehand stroke is a path with one point for each sample', () => {
  const stroke = { id: 's', brush: 'pen', color: '#f00', width: 3, points: [{ x: 1, y: 2 }, { x: 3, y: 4 }, { x: 5, y: 6 }] }
  assert.equal(strokePath(stroke), 'M 1 2 L 3 4 L 5 6')
  assert.equal(strokePath({ ...stroke, points: [{ x: 7, y: 8 }] }), 'M 7 8 L 7.01 8')
  assert.equal(strokePath({ ...stroke, points: [] }), '')
})

test('two suppliers of one field get a qualified name, and the chip survives a round trip', () => {
  const c = canvas([link('a', 'b'), link('b', 'c')], {
    triggerFields: [field('when')],
    flowFields: { a: [field('count', 'A')], b: [field('count', 'B'), field('priority', 'B')] },
  })
  const qualified = fieldsForStep(c, botKey('c'), true)
  assert.deepEqual(qualified.map((f) => f.name), ['when', 'B.count', 'priority'])
  assert.equal(qualifiedFieldName(qualified[1]!), 'B.count')
  const text = 'Use {{B.count}} now'
  assert.deepEqual(fieldTokens(text).map((token) => token.kind === 'field' ? token.name : token.text), ['Use ', 'B.count', ' now'])
  assert.equal(fieldTokens(text).map((token) => token.kind === 'field' ? token.raw : token.text).join(''), text)
  assert.deepEqual(chipBefore('Sum {{B.count}}', 15), { start: 4, end: 15, name: 'B.count' })
  assert.deepEqual(fieldTrigger('{{B.co', 6), { start: 0, query: 'B.co', single: false })
  assert.equal(fieldTokens('{{bad.name.x}}').some((token) => token.kind === 'field'), false, 'one qualifier only')
})

test('a run row labels its source, number, attempt, branch and waiting state', () => {
  const tr = (text: string) => text
  const run: WorkflowRun = {
    id: 'r1', workflowId: 'w', number: 12, triggerNodeId: null, source: 'github', status: 'done',
    error: null, startedAt: '1', endedAt: '2',
  }
  assert.equal(runSummary(run, 3, tr), '#12 · GitHub · 3 steps')
  const step: Parameters<typeof stepMarks>[0] = {
    id: 's', workflowId: 'w', runId: 'r1', nodeId: 'n', attempt: 2, agentId: null, status: 'running',
    output: null, branch: 'yes', error: null, startedAt: null, endedAt: null, waiting: true,
  }
  assert.deepEqual(stepMarks(step), { attempt: '#2', branch: 'yes', waiting: true })
  assert.deepEqual(stepMarks({ ...step, attempt: 1, branch: null, waiting: false }), { attempt: undefined, branch: undefined, waiting: false })
  assert.equal(nextRunCursor({ runs: [], soloSteps: [], nextBefore: 'abc' }), 'abc')
  assert.equal(nextRunCursor({ runs: [], soloSteps: [], nextBefore: null }), null)
  assert.equal(nextRunCursor(null), null)
  assert.equal(historyStatus('approval'), 'queued')
  assert.equal(historyStatus('done'), 'succeeded')
  assert.equal(historyStatus('stopping'), 'running')
  assert.equal(historyStatus('stopped'), 'cancelled')
  assert.equal(historyStatus('unknown'), 'queued')
})
