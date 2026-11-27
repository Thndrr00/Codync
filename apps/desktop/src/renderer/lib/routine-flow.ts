// Routine flows (host/src/routines/flow/, docs/features/routine-flows.md). The host owns the rules:
// it validates every change and runs the flow. This file only shapes what the canvas shows: node
// positions (saved, else automatic columns), edges, the list that mirrors the canvas, and a
// pre-check of the connect rules so a drop that the host would refuse never leaves the app.
// Media nodes follow OpenBot 0.70.0 `packages/ui/src/features/workflows/WorkflowImageNode.tsx`
// (a step that makes a file is a node like any other, linked to the steps around it).

import type { MediaKind, MediaModel } from '@shared/media'
import type { SharedFile } from '@shared/models'
import { isActiveRun, type Routine, type RoutineRun } from '../views/routines/routine.ts'

export type StepStatus = 'queued' | 'running' | 'succeeded' | 'failed' | 'skipped' | 'cancelled'

export interface FlowLink {
  id: string
  routineId: string
  fromBotId: string
  toBotId: string
  /** A name the person gave this step; empty or absent falls back to the step type. */
  name?: string
  instruction: string
  /** The model this step runs on instead of the next bot's own, and its options (effort…). */
  model?: string | null
  modelOptions?: Record<string, string>
  createdAt: number
}

export interface FlowStep {
  id: string
  runId: string
  botId: string
  deliveryId: string | null
  input: string
  output: string | null
  status: StepStatus
  error: string | null
  /** The pictures or clips a media step made, as shared files of the routine's bot. */
  files?: SharedFile[]
  /**
   * One entry per result of a batched media step (the host's `BatchResult`: index, status, error).
   * Older hosts send none, so the step itself is then the one result. A batch can mix pending,
   * failed and finished results; the files stay on the step, in result order.
   */
  results?: FlowMediaResult[]
  createdAt: number
  updatedAt: number
}

/** One result's state on a step, as the host records it while a batch runs. */
export interface FlowMediaResult {
  index: number
  status: 'pending' | 'succeeded' | 'failed'
  error?: string | null
}

/** One input a step read, as saved on the host with the settings of its run. */
export interface FlowStepInput {
  from: string
  text: string
  instruction: string
  /** A picture handed on by a media step, rather than a bot's text reply. */
  picture: boolean
}

/** What a step actually ran with; the same shape as `engine::StepSettings` on the host. */
export interface FlowStepSettings {
  /** `bot` or a media kind (`image`/`video`). */
  kind: string
  model?: string | null
  modelOptions?: Record<string, string>
  instruction?: string
  prompt?: string
  aspect?: string
  duration?: number | null
  resolution?: string
  transparent?: boolean | null
  voice?: string | null
  /** How many results a media run was asked for; older runs have none. */
  batch?: number | null
  /** One status per result, updated while a batch runs. */
  results?: FlowMediaResult[]
  inputs: FlowStepInput[]
}

/** A result with the files and price the result area needs. */
export interface FlowMediaTake extends FlowMediaResult {
  id: string
  files: SharedFile[]
  /** Credits for a Retry of this one result; null when the host sends no price. */
  credits: number | null
}

export interface FlowMedia {
  id: string
  /** A name the person gave this step; empty or absent falls back to the step kind. */
  name?: string
  kind: MediaKind
  model: string
  prompt: string
  aspect: string
  /** Seconds; only a video uses it. */
  duration: number
  /** Only a video uses it. */
  resolution: string
  transparent: boolean
  voice: string | null
  /** How many files one run makes, 1–4; an older host omits it and makes one. */
  batch?: number
  createdAt: number
  /** The routine the step belongs to. */
  routineId: string
}

export interface FlowPosition {
  nodeKey: string
  x: number
  y: number
}

export interface FlowRoutineCard {
  routine: Routine
  /** Whether this workflow skips its triggers; the host also stamps the canvas's paused ids. */
  paused?: boolean
  /** Newest first. */
  recentRuns: RoutineRun[]
  upcomingRuns: number[]
  /** The steps of the newest run that has any, oldest first. */
  steps: FlowStep[]
}

export interface FlowCanvas {
  botId: string
  rev: number
  routines: FlowRoutineCard[]
  links: FlowLink[]
  positions: FlowPosition[]
  placedBotIds: string[]
  /** The picture and clip steps of these routines (host/src/store/flow_media.rs). */
  mediaNodes: FlowMedia[]
  /** Settings each step ran with, by step id; a running batch updates its per-result states here. */
  stepSettings?: Record<string, FlowStepSettings>
  // Typed step fields and workflow actions (OpenBot nightly 0.70). All optional: an older host
  // simply omits them, and the canvas keeps working without completions, pauses or a registry.
  /** Fields the trigger gives; absent when the host doesn't know typed fields yet. */
  triggerFields?: FlowField[]
  /** Fields each step gives, by node key; the trigger's own key is ''. */
  flowFields?: Record<string, FlowField[]>
  /** Answer/check specs by host spec key; a newer host sends `{answer, check}` per key. */
  answerSpecs?: Record<string, string | { answer?: string; check?: string }>
  /** Check text by node key ('' = trigger). */
  checkSpecs?: Record<string, string>
  /** Problems by `${nodeKey}:${fieldName}`, or the host's list of `{nodeKey, text}`. */
  answerProblems?: Record<string, string> | readonly { nodeKey: string; text: string }[]
  /** Which setting values the host knows, by step kind then setting name. */
  nodeRegistry?: FlowNodeRegistry | null
  /** Routines whose workflow is paused: triggers are skipped, a manual run still works. */
  pausedRoutineIds?: string[]
  /** The host found a setting from a newer version: don't let this app save the workflow. */
  readOnly?: boolean
  /** The workflow document's format version; a newer one is read-only here. */
  formatVersion?: number
  // OpenBot-style workflow graph, when the host has the portable node/edge/stroke format. Older
  // hosts send none of these and the routine/link/media canvas keeps working as before.
  workflowGraph?: WorkflowGraph
  workflowTriggers?: WorkflowTriggerState[]
  needsNewerApp?: boolean
  workflowFormat?: number
  hostFormat?: number
  /** Edge problems the host found (`annotation`, `port`, `branch`, `condition-branch`, `duplicate`). */
  graphProblems?: Record<string, string>
  /** The newest history step of each portable node (`approval` steps carry `waiting`). */
  workflowSteps?: Record<string, WorkflowHistoryStep>
}

/** The graph a canvas shows, in either wire shape; null on an older host. */
export function canvasGraph(canvas: FlowCanvas | null): WorkflowGraph | null {
  return canvas?.workflowGraph ?? null
}

/** The workflow format this client knows (host/src/routines/flow/format.rs). */
export const FLOW_FORMAT_VERSION = 1

/** One folder of the computer-wide "All workflows" tree (host `workflowFolders`). */
export interface WorkflowFolder {
  id: string
  name: string
}

/** The host's folder tree: folders and which workflow sits in which one. */
export interface WorkflowFoldersSnapshot {
  revision: number
  folders: WorkflowFolder[]
  placement: Record<string, string>
}

/** The body of `workflowFolderApply`; `action` is the discriminator (OpenBot's folder actions). */
export type WorkflowFolderAction =
  | { action: 'create'; id: string; name: string; workflowIds?: string[] }
  | { action: 'rename'; folderId: string; name: string }
  | { action: 'delete'; folderId: string }
  | { action: 'place'; workflowIds: string[]; folderId: string | null }

/** The host's limit on how far a flow reaches from its routine's bot. */
export const DEFAULT_MAX_DEPTH = 5
export const MAX_COORDINATE = 1_000_000
/** The size a node card takes on the canvas; the columns and the edge anchors follow it. */
export const NODE_WIDTH = 220
export const NODE_HEIGHT = 120
const COLUMN_GAP = 90
const ROW_GAP = 40

/** A stable hue per routine, so its nodes and edges read as one path. */
export function routineTint(id: string): string {
  let hash = 0
  for (const char of id) hash = (hash * 31 + (char.codePointAt(0) ?? 0)) % 360
  return `hsl(${hash} 58% 58%)`
}

export const routineKey = (id: string) => `routine:${id}`
export const botKey = (id: string) => `bot:${id}`
export const mediaKey = (id: string) => `media:${id}`

/** The canvas key of a link's end, which names a bot or a `media:` step. */
export const flowKey = (id: string) => (id.startsWith('media:') ? mediaKey(id.slice(6)) : botKey(id))

/** The media step a link end names, if it is one. */
export function mediaEnd(id: string): string | null {
  return id.startsWith('media:') ? id.slice(6) : null
}

/** The media step of this canvas a node id stands for. */
export function mediaOf(canvas: FlowCanvas, id: string): FlowMedia | null {
  const own = mediaEnd(id)
  return own === null ? null : (canvas.mediaNodes.find((media) => media.id === own) ?? null)
}

/** What a media node is called: its own description, else its kind. */
export function mediaTitle(media: FlowMedia | null): string {
  const prompt = media?.prompt.trim()
  if (prompt) return prompt
  return media?.kind === 'video' ? 'Video' : 'Picture'
}

// MARK: media step batch, price, availability and results (OpenBot 0.70+)

/** The batch sizes an image or clip step can make at once (OpenBot's `MEDIA_BATCH_SIZES`). */
export const MEDIA_BATCH_SIZES = [1, 2, 3, 4] as const
export type MediaBatch = (typeof MEDIA_BATCH_SIZES)[number]
export const MAX_MEDIA_TAKES = 4

/** A batch value from the host or the editor: only 1–4 count, anything else is one result. */
export function mediaBatch(value: unknown): MediaBatch {
  return MEDIA_BATCH_SIZES.find((size) => size === value) ?? 1
}

/** What the flow reads of the host's `mediaModels` answer. The new fields are optional on the wire. */
export interface FlowMediaCatalog {
  agent: string | null
  installed: boolean
  models: FlowMediaModel[]
  /** True once the host loaded its live list: only then is an unknown model "unavailable". */
  modelsReady?: boolean
  /** Names of models the service can't make now, for a saved step that still names one. */
  unavailableModels?: { id: string; name: string; kinds: string[] }[]
  /** One result's price with the chosen settings, when the host prices the answer itself. */
  price?: number | null
}

/** A catalog model: `available` is the host's live flag, `batchCredits` its priced batch. */
export type FlowMediaModel = MediaModel & {
  available?: boolean
  credits?: number | null
  batchCredits?: { credits?: number | null }
}

/** A saved model the service can't make now, so its name stays on the button. */
export function unavailableMediaModel(
  answer: FlowMediaCatalog | null,
  modelId: string,
  kind: MediaKind,
): { id: string; name: string } | null {
  if (!modelId) return null
  const listed = answer?.models.find((model) => model.id === modelId && model.kinds.includes(kind))
  const named = answer?.unavailableModels?.find((model) => model.id === modelId && model.kinds.includes(kind))
  if (listed) {
    return listed.available === false || named ? (named ?? listed) : null
  }
  // A model missing from a loaded list is unavailable too; only an empty older answer says nothing.
  if (named || answer?.modelsReady || (answer?.models.length ?? 0) > 0) {
    return named ?? { id: modelId, name: modelId }
  }
  return null
}

/** The saved model is unavailable: a run reaching this step stops with the host's error. */
export function missingMediaModel(answer: FlowMediaCatalog | null, modelId: string, kind: MediaKind): boolean {
  return unavailableMediaModel(answer, modelId, kind) !== null
}

/**
 * Models offered in the picker: the host's available ones only, so a saved unavailable model drops
 * out of the list while its button keeps its name and marker. The media switch-all change needs no
 * UI beyond this flag — switching to another available model uses the same picker.
 */
export function availableMediaModels(answer: FlowMediaCatalog | null, kind: MediaKind): MediaModel[] {
  return answer?.models.filter((model) => model.kinds.includes(kind) && model.available !== false) ?? []
}

/** Credits for one result, from the host's `batchCredits.credits` (or an older answer), when priced. */
export function mediaUnitPrice(answer: FlowMediaCatalog | null, modelId: string): number | null {
  if (typeof answer?.price === 'number' && Number.isFinite(answer.price)) return answer.price
  const model = answer?.models.find((choice) => choice.id === modelId)
  const credits = model?.batchCredits?.credits ?? model?.credits
  return typeof credits === 'number' && Number.isFinite(credits) ? credits : null
}

/** The price the Run button shows: one result's price times the batch, or null for no price. */
export function mediaTotalCredits(price: number | null, batch: number): number | null {
  return price !== null && Number.isFinite(price) ? price * mediaBatch(batch) : null
}

type Tr = (text: string) => string

/** The Run button's words: priced when the host sends a price, and the host's marker when it cannot make the model. */
export function mediaRunLabel(price: number | null, batch: number, missing: boolean, tr: Tr): string {
  if (missing) return tr('Model not available')
  const credits = mediaTotalCredits(price, batch)
  const base = credits === null ? tr('Run') : tr('Run · {credits} credits').replace('{credits}', String(credits))
  const takes = mediaBatch(batch)
  return takes > 1 ? `${base} · ${tr('{count} results').replace('{count}', String(takes))}` : base
}

/** A failed take's Retry button, priced for one result rather than the whole batch. */
export function mediaRetryLabel(price: number | null, tr: Tr): string {
  return price === null ? tr('Retry') : tr('Retry · {credits} credits').replace('{credits}', String(price))
}

/** The newest steps a node left, oldest first and at most one batch; older hosts have just one. */
export function mediaStepsFor(canvas: FlowCanvas, nodeKey: string): FlowStep[] {
  return canvas.routines
    .flatMap((card) => card.steps)
    .filter((step) => step.botId === nodeKey)
    .map((step) => {
      // The host keeps a batch's progress on the step's saved settings, not on the step row.
      const results = canvas.stepSettings?.[step.id]?.results ?? step.results
      return results?.length ? { ...step, results } : step
    })
    .sort((a, b) => a.updatedAt - b.updatedAt)
    .slice(-MAX_MEDIA_TAKES)
}

/**
 * The results of a batch by state, plus one file list for the shared prev/next thumbnails. A
 * finished result owns the next file in step order, so a failed result in between does not shift
 * which file belongs to which success. An older step without per-result states is one result.
 */
export function groupMediaResults(steps: readonly FlowStep[], price: number | null): {
  pending: FlowMediaTake[]
  failed: FlowMediaTake[]
  succeeded: FlowMediaTake[]
  files: SharedFile[]
} {
  const takes: FlowMediaTake[] = []
  for (const step of steps) {
    const files = step.files ?? []
    const single = !step.results?.length
    // A model-unavailable run fails before any take starts: the host's step error is the result.
    const stopped = !single && step.status === 'failed' && !!step.error
      && !step.results!.some((result) => result.status === 'failed')
    const listed: { index: number; status: FlowMediaResult['status'] | null; error?: string | null }[] = single || stopped
      ? [{
        index: 0,
        status: step.status === 'running' ? 'pending'
          : step.status === 'failed' || step.status === 'succeeded' ? step.status : null,
        error: step.error,
      }]
      : step.results ?? []
    let fileAt = 0
    for (const result of listed) {
      const status = result.status
      if (status !== 'pending' && status !== 'failed' && status !== 'succeeded') continue
      const made = status === 'succeeded'
        ? (single ? files : (files[fileAt] ? [files[fileAt]] : []))
        : []
      if (made.length > 0 && !single) fileAt += 1
      takes.push({ index: result.index, status, error: result.error, id: `${step.id}:${result.index}`, files: made, credits: price })
    }
  }
  const recent = takes.slice(-MAX_MEDIA_TAKES)
  const pending = recent.filter((take) => take.status === 'pending')
  const failed = recent.filter((take) => take.status === 'failed')
  const succeeded = recent.filter((take) => take.status === 'succeeded')
  const seen = new Set<string>()
  const files: SharedFile[] = []
  for (const take of succeeded) {
    for (const file of take.files) {
      if (seen.has(file.id)) continue
      seen.add(file.id)
      files.push(file)
    }
  }
  return { pending, failed, succeeded, files }
}

export interface FlowNode {
  key: string
  kind: 'routine' | 'bot' | 'media' | 'workflow'
  /** The routine's, the bot's or the media step's id (`media:` steps keep their prefix). */
  id: string
  x: number
  y: number
  /** Whether the position was saved on the host (else it is the automatic column). */
  saved: boolean
}

export interface FlowEdge {
  id: string
  from: string
  to: string
  /** The link this edge is; null for a routine's fixed edge to its own bot. */
  linkId: string | null
}

export const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value))
export const clampCoordinate = (value: number) => clamp(Math.round(value), -MAX_COORDINATE, MAX_COORDINATE)

/** The bot that starts a routine's flow. */
export function ownerOf(canvas: FlowCanvas, routineId: string): string | null {
  return canvas.routines.find((card) => card.routine.id === routineId)?.routine.botId ?? null
}

/** How many hops from the routine's own bot each bot of its flow is (the longest path); absent when not reached. */
export function depths(owner: string, links: readonly FlowLink[]): Map<string, number> {
  const depth = new Map<string, number>([[owner, 0]])
  // The host refuses cycles, so a depth only grows, at most once per link; the cap keeps a bad canvas from spinning.
  for (let round = 0; round <= links.length; round++) {
    let moved = false
    for (const link of links) {
      const from = depth.get(link.fromBotId)
      if (from === undefined || (depth.get(link.toBotId) ?? -1) >= from + 1) continue
      depth.set(link.toBotId, from + 1)
      moved = true
    }
    if (!moved) break
  }
  return depth
}

function reaches(links: readonly FlowLink[], start: string, target: string): boolean {
  const seen = new Set<string>()
  const queue = [start]
  for (let next = queue.pop(); next !== undefined; next = queue.pop()) {
    if (next === target) return true
    if (seen.has(next)) continue
    seen.add(next)
    for (const link of links) if (link.fromBotId === next) queue.push(link.toBotId)
  }
  return false
}

function longestFrom(links: readonly FlowLink[], bot: string, seen = new Set<string>()): number {
  if (seen.has(bot)) return 0
  seen.add(bot)
  let best = 0
  for (const link of links) if (link.fromBotId === bot) best = Math.max(best, 1 + longestFrom(links, link.toBotId, seen))
  seen.delete(bot)
  return best
}

/** Why `from` can't hand a routine's reply to `to` (the host's rules, in its words), or null when it can. */
export function connectProblem(canvas: FlowCanvas, routineId: string, from: string, to: string, maxDepth = DEFAULT_MAX_DEPTH): string | null {
  const owner = ownerOf(canvas, routineId)
  if (!owner) return 'That routine is not on this canvas.'
  const links = canvas.links.filter((link) => link.routineId === routineId)
  if (from === to) return "A bot can't hand work to itself."
  if (to === owner) return "The routine's own bot starts the flow, so nothing can hand work back to it."
  const media = mediaOf(canvas, to)
  if (media && media.routineId !== routineId) return 'That step is not part of this routine.'
  const reached = depths(owner, links)
  const here = reached.get(from)
  if (here === undefined) return "The flow doesn't reach that bot yet: connect it from the routine's bot first."
  if (!media && !canvasBotIds(canvas).includes(to)) return 'That bot is not on this canvas.'
  if (links.some((link) => link.fromBotId === from && link.toBotId === to)) return 'These two bots are already connected in this routine.'
  if (reaches(links, to, from)) return 'That link would make the flow loop.'
  if (here + 1 + longestFrom(links, to) > maxDepth) return `A flow reaches at most ${maxDepth} bots deep.`
  return null
}

/** The bots a canvas shows: the canvas's own, every routine's bot, every link's bot ends and the bots placed by hand. */
export function canvasBotIds(canvas: FlowCanvas): string[] {
  const ids = new Set<string>([canvas.botId])
  const add = (id: string) => {
    if (!id.startsWith('media:')) ids.add(id)
  }
  for (const card of canvas.routines) add(card.routine.botId)
  for (const link of canvas.links) {
    add(link.fromBotId)
    add(link.toBotId)
  }
  for (const id of canvas.placedBotIds) add(id)
  return [...ids]
}

/**
 * Every node with its place: the saved one, else an automatic column. Routines stand in the first
 * column; a bot or a media step goes one column right of the deepest routine flow that reaches it,
 * nodes no flow reaches (placed by hand without a link) in the last column. A node never moved
 * keeps its column as the canvas grows, so the layout is stable.
 */
export function layoutNodes(canvas: FlowCanvas): FlowNode[] {
  const saved = new Map(canvas.positions.map((p) => [p.nodeKey, p]))
  const column = new Map<string, number>()
  for (const card of canvas.routines) {
    const reached = depths(card.routine.botId, canvas.links.filter((l) => l.routineId === card.routine.id))
    for (const [node, depth] of reached) column.set(node, Math.max(column.get(node) ?? 0, depth + 1))
  }
  const lastColumn = Math.max(1, ...column.values()) + 1
  const rows = new Map<number, number>()
  const place = (key: string, kind: FlowNode['kind'], id: string, col: number): FlowNode => {
    const position = saved.get(key)
    const row = rows.get(col) ?? 0
    rows.set(col, row + 1)
    const x = position?.x ?? col * (NODE_WIDTH + COLUMN_GAP)
    const y = position?.y ?? row * (NODE_HEIGHT + ROW_GAP)
    return { key, kind, id, x, y, saved: position !== undefined }
  }
  const nodes = canvas.routines.map((card) => place(routineKey(card.routine.id), 'routine', card.routine.id, 0))
  for (const id of canvasBotIds(canvas)) nodes.push(place(botKey(id), 'bot', id, column.get(id) ?? lastColumn))
  for (const media of canvas.mediaNodes) {
    const id = mediaKey(media.id)
    nodes.push(place(id, 'media', id, column.get(id) ?? lastColumn))
  }
  return nodes
}

/** A drawing-only group's members, which tidy must move as one block. */
export interface TidyGroup {
  id: string
  members: readonly string[]
}

/** A tidy canvas: every node's new place, and the new origin of each group's card. */
export interface TidyLayout {
  positions: FlowPosition[]
  groups: { id: string; x: number; y: number }[]
}

/**
 * Puts a canvas in columns from left to right in the order the steps run, with each branch on its
 * own row below its siblings. A group's members occupy consecutive rows of one column and move as
 * one block, and its card follows them. Nothing depends on saved positions, so "Tidy up" is the
 * same arrangement every time; the view keeps the old places to offer Undo. The host holds no
 * locked steps or free-floating notes on this canvas, so there is nothing to leave behind.
 */
export function tidyLayout(canvas: FlowCanvas, groups: readonly TidyGroup[] = []): TidyLayout {
  const nodes: { key: string; col: number }[] = canvas.routines.map((card) => ({ key: routineKey(card.routine.id), col: 0 }))
  const column = new Map<string, number>()
  for (const card of canvas.routines) {
    const reached = depths(card.routine.botId, canvas.links.filter((l) => l.routineId === card.routine.id))
    for (const [id, depth] of reached) {
      const key = id.startsWith('media:') ? id : botKey(id)
      column.set(key, Math.max(column.get(key) ?? 0, depth + 1))
    }
  }
  const lastColumn = Math.max(1, ...column.values()) + 1
  for (const id of canvasBotIds(canvas)) nodes.push({ key: botKey(id), col: column.get(botKey(id)) ?? lastColumn })
  for (const media of canvas.mediaNodes) {
    const key = mediaKey(media.id)
    nodes.push({ key, col: column.get(key) ?? lastColumn })
  }
  // Run order comes from the same depth-first walk the list uses; nodes placed by hand come last.
  const order = new Map<string, number>()
  for (const row of flowRows(canvas)) {
    const key = row.kind === 'media' ? row.nodeId : botKey(row.nodeId)
    if (!order.has(key)) order.set(key, order.size)
  }
  const sequence = new Map(nodes.map((node, index) => [node.key, index]))
  const sorted = [...nodes].sort((a, b) => a.col - b.col || (order.get(a.key) ?? Infinity) - (order.get(b.key) ?? Infinity) || sequence.get(a.key)! - sequence.get(b.key)!)

  const byKey = new Map(sorted.map((node) => [node.key, node]))
  const groupOfNode = new Map<string, TidyGroup>()
  for (const group of groups) for (const key of group.members) groupOfNode.set(key, group)
  const units: { keys: string[]; col: number }[] = []
  const placed = new Set<string>()
  const usedGroups = new Set<string>()
  for (const node of sorted) {
    if (placed.has(node.key)) continue
    const group = groupOfNode.get(node.key)
    if (group && !usedGroups.has(group.id)) {
      usedGroups.add(group.id)
      const keys = group.members.filter((key) => byKey.has(key))
      for (const key of keys) placed.add(key)
      units.push({ keys, col: Math.min(...keys.map((key) => byKey.get(key)!.col)) })
    } else if (!group) {
      placed.add(node.key)
      units.push({ keys: [node.key], col: node.col })
    }
  }

  const rows = new Map<number, number>()
  const groupAt = new Map<string, { x: number; y: number }>()
  const positions: FlowPosition[] = []
  for (const unit of units) {
    const row = rows.get(unit.col) ?? 0
    rows.set(unit.col, row + unit.keys.length)
    unit.keys.forEach((key, index) => {
      const x = unit.col * (NODE_WIDTH + COLUMN_GAP)
      const y = (row + index) * (NODE_HEIGHT + ROW_GAP)
      positions.push({ nodeKey: key, x, y })
      const group = groupOfNode.get(key)
      if (group && !groupAt.has(group.id)) groupAt.set(group.id, { x, y })
    })
  }
  return { positions, groups: [...groupAt].map(([id, at]) => ({ id, ...at })) }
}

/** Each routine's fixed edge to its bot, then the links (whose ends are bots or media steps). */
export function flowEdges(canvas: FlowCanvas): FlowEdge[] {
  const starts = canvas.routines.map((card) => ({ id: `start:${card.routine.id}`, from: routineKey(card.routine.id), to: botKey(card.routine.botId), linkId: null }))
  return [...starts, ...canvas.links.map((link) => ({ id: link.id, from: flowKey(link.fromBotId), to: flowKey(link.toBotId), linkId: link.id }))]
}

/** The latest step the canvas holds for a bot, across its routines. */
export function latestStep(canvas: FlowCanvas, botId: string): FlowStep | null {
  let best: FlowStep | null = null
  for (const card of canvas.routines) {
    for (const step of card.steps) {
      if (step.botId === botId && (!best || step.updatedAt >= best.updatedAt)) best = step
    }
  }
  return best
}

export function stepOf(card: FlowRoutineCard, botId: string): FlowStep | null {
  return card.steps.find((step) => step.botId === botId) ?? null
}

/** The id a link uses for a canvas node: `bot:id` drops its prefix, `media:id` keeps it. */
export function linkEndOf(nodeKey: string): string {
  return nodeKey.startsWith('bot:') ? nodeKey.slice(4) : nodeKey
}

/** A step's own name, or '' when it has none: callers then fall back to the step type. */
export function stepNameOf(canvas: FlowCanvas, nodeKey: string): string {
  if (nodeKey.startsWith('media:')) {
    const media = canvas.mediaNodes.find((node) => mediaKey(node.id) === nodeKey)
    return media?.name?.trim() ?? ''
  }
  const link = canvas.links.find((candidate) => candidate.toBotId === linkEndOf(nodeKey))
  return link?.name?.trim() ?? ''
}

/** What a node's output port carries, which decides the soft tint of its connected line. */
export function portCarry(node: FlowNode, media: FlowMedia | null): 'text' | 'image' | 'file' {
  if (node.kind === 'media') return media?.kind === 'video' ? 'file' : 'image'
  return 'text'
}

/** What a running step shows while its agent works: its predecessor, the count done and the total. */
export function stepProgress(card: FlowRoutineCard, step: FlowStep): { previous: FlowStep | null; done: number; total: number } {
  const index = card.steps.findIndex((candidate) => candidate.id === step.id)
  return {
    previous: index > 0 ? card.steps[index - 1] : null,
    done: card.steps.filter((candidate) => candidate.status !== 'running' && candidate.status !== 'queued').length,
    total: card.steps.length,
  }
}

/** One line of the list that mirrors the canvas: a routine, or a bot or media step under the step that hands to it. */
export interface FlowRow {
  key: string
  routineId: string
  /** The bot id, or the `media:` step id. */
  nodeId: string
  kind: 'bot' | 'media'
  media: FlowMedia | null
  depth: number
  link: FlowLink | null
  step: FlowStep | null
}

const MAX_ROWS = 200

/** The canvas as a list, routine by routine and depth first: what the keyboard and a screen reader walk. */
export function flowRows(canvas: FlowCanvas): FlowRow[] {
  const rows: FlowRow[] = []
  for (const card of canvas.routines) {
    const links = canvas.links.filter((l) => l.routineId === card.routine.id)
    const walk = (nodeId: string, depth: number, link: FlowLink | null, path: readonly string[]) => {
      if (rows.length >= MAX_ROWS || path.includes(nodeId)) return
      const media = mediaOf(canvas, nodeId)
      rows.push({
        key: `${card.routine.id}/${link?.id ?? 'start'}`,
        routineId: card.routine.id,
        nodeId,
        kind: media ? 'media' : 'bot',
        media,
        depth,
        link,
        step: stepOf(card, nodeId),
      })
      for (const next of links.filter((l) => l.fromBotId === nodeId)) walk(next.toBotId, depth + 1, next, [...path, nodeId])
    }
    walk(card.routine.botId, 0, null, [])
  }
  return rows
}

/** Whether this canvas has a step still running: the run snapshot a workflow tab's dot reads. */
export function canvasRunning(canvas: FlowCanvas | null): boolean {
  return !!canvas && canvas.routines.some((card) =>
    card.steps.some((step) => step.status === 'running' || step.status === 'queued') || card.recentRuns.some(isActiveRun))
}

/** The canvas point at the middle of the viewport: where steps pasted from elsewhere land. */
export function canvasCenter(view: View, width: number, height: number): { x: number; y: number } {
  return { x: (width / 2 - view.x) / view.scale, y: (height / 2 - view.y) / view.scale }
}

/**
 * Where a node added from outside the canvas lands: the middle of what the person is looking at,
 * with its own box subtracted so the whole card is on screen. The view's pan and zoom decide it,
 * never a saved position from a different workflow (OpenBot 0.70 workflow node fix).
 */
export function visible_spot(view: View, box: { width: number; height: number }): { x: number; y: number } {
  const center = canvasCenter(view, box.width, box.height)
  return { x: Math.round(center.x - NODE_WIDTH / 2), y: Math.round(center.y - NODE_HEIGHT / 2) }
}

/** A workflow's folder id, or null when it is unfiled or the folder has gone. */
export function folderOf(snapshot: WorkflowFoldersSnapshot | null, workflowId: string): string | null {
  const folderId = snapshot?.placement[workflowId]
  return folderId && snapshot?.folders.some((folder) => folder.id === folderId) ? folderId : null
}

/** The given workflows filed in `folderId` (null = unfiled), keeping their order. */
export function workflowsInFolder(
  snapshot: WorkflowFoldersSnapshot | null,
  ids: readonly string[],
  folderId: string | null,
): string[] {
  return ids.filter((id) => folderOf(snapshot, id) === folderId)
}

/** What a step's status says, in the words the other clients use. */
export function statusLabel(status: StepStatus | null): string {
  switch (status) {
    case 'queued': return 'Queued'
    case 'running': return 'Running'
    case 'succeeded': return 'Succeeded'
    case 'failed': return 'Failed'
    case 'skipped': return 'Skipped'
    case 'cancelled': return 'Cancelled'
    default: return 'Not run yet'
  }
}

/** The canvas's empty state: nothing has run for it, or nothing is connected yet. */
export function isUnconnected(canvas: FlowCanvas, routineId: string): boolean {
  return !canvas.links.some((link) => link.routineId === routineId)
}

// MARK: view (pan and zoom)

export interface View {
  x: number
  y: number
  scale: number
}

export const MIN_SCALE = 0.3
export const MAX_SCALE = 2

/** Zoom by `factor` keeping the point (`px`, `py`) of the viewport still. */
export function zoomAt(view: View, factor: number, px: number, py: number): View {
  const scale = clamp(view.scale * factor, MIN_SCALE, MAX_SCALE)
  const k = scale / view.scale
  return { scale, x: px - (px - view.x) * k, y: py - (py - view.y) * k }
}

/** The view that shows every node inside a viewport of `width` × `height`, with a margin. */
export function fitView(nodes: readonly FlowNode[], width: number, height: number, margin = 40): View {
  if (nodes.length === 0 || width <= 0 || height <= 0) return { x: 0, y: 0, scale: 1 }
  const left = Math.min(...nodes.map((n) => n.x))
  const top = Math.min(...nodes.map((n) => n.y))
  const right = Math.max(...nodes.map((n) => n.x + NODE_WIDTH))
  const bottom = Math.max(...nodes.map((n) => n.y + NODE_HEIGHT))
  const scale = clamp(Math.min((width - 2 * margin) / (right - left), (height - 2 * margin) / (bottom - top)), MIN_SCALE, 1)
  return { scale, x: (width - (right - left) * scale) / 2 - left * scale, y: (height - (bottom - top) * scale) / 2 - top * scale }
}

/** Where an edge leaves a node (its right middle) and enters the next (its left middle). */
export function edgePath(from: FlowNode, to: FlowNode): string {
  const x1 = from.x + NODE_WIDTH
  const y1 = from.y + NODE_HEIGHT / 2
  const x2 = to.x
  const y2 = to.y + NODE_HEIGHT / 2
  const bend = Math.max(40, Math.abs(x2 - x1) / 2)
  return `M ${x1} ${y1} C ${x1 + bend} ${y1}, ${x2 - bend} ${y2}, ${x2} ${y2}`
}

/** The node whose card holds the canvas point, if any: where a dragged connection was dropped. */
export function nodeAt(nodes: readonly FlowNode[], x: number, y: number): FlowNode | null {
  return nodes.find((n) => x >= n.x && x <= n.x + NODE_WIDTH && y >= n.y && y <= n.y + NODE_HEIGHT) ?? null
}

// MARK: typed step fields (OpenBot nightly workflow fields)

export type FlowFieldType = 'text' | 'number' | 'integer' | 'yesNo' | 'list' | 'choice'

/** One field an earlier step (or the trigger) gives, as the host sends it. */
export interface FlowField {
  name: string
  /** The giving step's name; empty for the trigger. */
  step: string
  /** SF Symbol of the giving step, shown on the field list's one line. */
  icon: string
  type: FlowFieldType
  /** Values of a "One of" field; empty for the other types. */
  values: string[]
}

export type FieldToken = { kind: 'text'; text: string } | { kind: 'field'; name: string; raw: string }

/** A bare field name, so a `{{...}}` in JSON or code is never mistaken for one. */
// A `step.name` qualifier disambiguates two steps that give the same field (OpenBot 0.70).
const FIELD_NAME = /^[A-Za-z_][A-Za-z0-9_-]*(?:\.[A-Za-z_][A-Za-z0-9_-]*)?$/

/**
 * Splits a saved prompt into plain text and `{{name}}` fields. The raw braces stay in the pieces so
 * a save is a lossless join; callers render a field as a chip. An unclosed or malformed pair is text.
 */
export function fieldTokens(text: string): FieldToken[] {
  const tokens: FieldToken[] = []
  let at = 0
  while (at < text.length) {
    const open = text.indexOf('{{', at)
    if (open < 0) break
    const close = text.indexOf('}}', open + 2)
    if (close < 0) break
    const name = text.slice(open + 2, close)
    if (FIELD_NAME.test(name)) {
      if (open > at) tokens.push({ kind: 'text', text: text.slice(at, open) })
      tokens.push({ kind: 'field', name, raw: text.slice(open, close + 2) })
      at = close + 2
    } else {
      // A brace pair that isn't a field: keep scanning after it for a real one.
      tokens.push({ kind: 'text', text: text.slice(at, open + 2) })
      at = open + 2
    }
  }
  if (at < text.length) tokens.push({ kind: 'text', text: text.slice(at) })
  return tokens
}

/** The whole chip right before `caret`, so Backspace can remove it in one go. */
export function chipBefore(text: string, caret: number): { start: number; end: number; name: string } | null {
  if (caret < 4 || text[caret - 1] !== '}' || text[caret - 2] !== '}') return null
  const start = text.lastIndexOf('{{', caret - 2)
  if (start < 0) return null
  const name = text.slice(start + 2, caret - 2)
  return FIELD_NAME.test(name) ? { start, end: caret, name } : null
}

/** An open completion at the caret: `{{que` or a single `{`. */
export interface FieldTrigger {
  /** Where the trigger starts, so the field token replaces all of the typed braces and letters. */
  start: number
  query: string
  /** A single `{` (not `{{`) — Enter still means a new line until a letter or arrow is used. */
  single: boolean
}

/** The completion trigger before the caret, or null in ordinary text (JSON, code, a closed chip). */
export function fieldTrigger(text: string, caret: number): FieldTrigger | null {
  const brace = text.lastIndexOf('{', caret - 1)
  if (brace < 0) return null
  const single = text[brace - 1] !== '{'
  const start = single ? brace : brace - 1
  if (start < 0) return null
  // A single brace stuck to a word is part of that word, not a trigger.
  if (single && brace > 0 && /[\p{L}\p{N}_]/u.test(text[brace - 1])) return null
  const query = text.slice(start + (single ? 1 : 2), caret)
  if (/[^\p{L}\p{N}_.-]/u.test(query)) return null
  return { start, query, single }
}

/** Replaces the typed trigger with `{{name}}` and puts the caret after it. Without a trigger, inserts at the caret. */
export function insertFieldToken(text: string, caret: number, name: string): { text: string; caret: number } {
  const trigger = fieldTrigger(text, caret)
  const start = trigger?.start ?? caret
  const token = `{{${name}}}`
  const before = text.slice(0, start)
  return { text: before + token + text.slice(caret), caret: before.length + token.length }
}

export interface TextFieldPart {
  text: string
  match: boolean
}
export interface FieldMatch {
  field: FlowField
  /** The name split around the typed letters, so the list can bold just those. */
  parts: TextFieldPart[]
}

/** Fields whose name holds the typed text, name-prefix matches first, then host order. */
export function matchingFields(fields: readonly FlowField[], query: string): FieldMatch[] {
  const q = query.trim().toLocaleLowerCase()
  if (!q) return fields.map((field) => ({ field, parts: [{ text: field.name, match: false }] }))
  const matches: FieldMatch[] = []
  for (const field of fields) {
    const lower = field.name.toLocaleLowerCase()
    const at = lower.indexOf(q)
    if (at < 0) continue
    const parts: TextFieldPart[] = []
    if (at > 0) parts.push({ text: field.name.slice(0, at), match: false })
    parts.push({ text: field.name.slice(at, at + q.length), match: true })
    if (at + q.length < field.name.length) parts.push({ text: field.name.slice(at + q.length), match: false })
    matches.push({ field, parts })
  }
  return matches.sort((a, b) =>
    Number(b.field.name.toLocaleLowerCase().startsWith(q)) - Number(a.field.name.toLocaleLowerCase().startsWith(q)))
}

/** The fields a step can name: the trigger's, then every earlier step's; the nearest step wins on a repeated name. */
export function fieldsForStep(canvas: FlowCanvas, nodeKey: string, qualify = false): FlowField[] {
  const rows = flowRows(canvas)
  const target = rows.findIndex((row) => (row.kind === 'media' ? mediaKey(row.nodeId) : botKey(row.nodeId)) === nodeKey)
  const earlier = target < 0 ? rows : rows.slice(0, target)
  const list = [...(canvas.triggerFields ?? [])]
  for (const row of earlier) list.push(...(canvas.flowFields?.[row.nodeId] ?? []))
  const suppliers = new Map<string, Set<string>>()
  for (const field of list) {
    const names = suppliers.get(field.name) ?? new Set<string>()
    names.add(field.step)
    suppliers.set(field.name, names)
  }
  const seen = new Set<string>()
  const fields = list.reverse().filter((field) => {
    if (seen.has(field.name)) return false
    seen.add(field.name)
    return true
  }).reverse()
  return qualify
    ? fields.map((field) => (suppliers.get(field.name)!.size > 1 && field.step
      ? { ...field, name: `${field.step}.${field.name}` }
      : field))
    : fields
}

/** The name inserted by the field picker: qualified when two suppliers gave the same field. */
export const qualifiedFieldName = (field: FlowField): string =>
  field.step && !field.name.includes('.') ? `${field.step}.${field.name}` : field.name

/** The answer or check text of a spec, whether the host sent a bare string or `{answer, check}`. */
export function specText(spec: string | { answer?: string; check?: string } | undefined, part: 'answer' | 'check'): string {
  if (!spec) return ''
  return typeof spec === 'string' ? (part === 'answer' ? spec : '') : spec[part] ?? ''
}

/** Host spec keys drop the `bot:` prefix of a step's canvas key. */
export function specKeyOf(nodeKey: string): string {
  return nodeKey.startsWith('bot:') ? nodeKey.slice(4) : nodeKey
}

function problemEntries(canvas: FlowCanvas): [string, string][] {
  const problems = canvas.answerProblems
  if (!problems) return []
  if (Array.isArray(problems)) return problems.map((problem) => [problem.nodeKey, problem.text])
  return Object.entries(problems)
}

/** Every chip problem on a step, by field name (`*` = the whole step), for the chips of its prompt. */
export function problemsForStep(canvas: FlowCanvas, nodeKey: string): Record<string, string> {
  const result: Record<string, string> = {}
  const nodeProblem = problemEntries(canvas).find(([key]) => key === nodeKey)?.[1]
  for (const [key, problem] of problemEntries(canvas)) {
    if (!key.includes(':')) {
      result[key] = problem
    } else if (key.startsWith(`${nodeKey}:`)) {
      result[key.slice(nodeKey.length + 1)] = problem
    }
  }
  // The host's problem list names the step, not the field: every chip on it shows the warning.
  if (nodeProblem) result['*'] = nodeProblem
  return result
}

/** Whether a routine's workflow skips its triggers (a manual run is still allowed). */
export function isFlowPaused(canvas: FlowCanvas, routineId: string): boolean {
  return canvas.pausedRoutineIds?.includes(routineId)
    ?? canvas.routines.find((card) => card.routine.id === routineId)?.paused
    ?? false
}

/** The names in a portable workflow file, read before the person maps them to local agents. */
export interface WorkflowFileInfo {
  name: string
  agents: string[]
  channels: string[]
  assets: number
}

/** The bytes to save: a plain JSON file is already text, an older host sends it base64-encoded. */
export function workflowFileBytes(content: string): Uint8Array<ArrayBuffer> {
  if (content.trimStart().startsWith('{')) return new TextEncoder().encode(content)
  const binary = atob(content.trim())
  return Uint8Array.from(binary, (char) => char.charCodeAt(0))
}

/** Reads a workflow file's JSON (plain or base64); null when the bytes aren't a workflow. */
export function parseWorkflowFile(raw: string): WorkflowFileInfo | null {
  let text = raw.trim()
  if (!text.startsWith('{')) {
    try {
      const binary = atob(text)
      text = new TextDecoder().decode(Uint8Array.from(binary, (char) => char.charCodeAt(0)))
    } catch {
      return null
    }
  }
  let file: Record<string, unknown>
  try {
    const parsed: unknown = JSON.parse(text)
    if (!parsed || typeof parsed !== 'object') return null
    file = parsed as Record<string, unknown>
  } catch {
    return null
  }
  const workflow = (file.workflow ?? file) as Record<string, unknown>
  if (!Array.isArray(workflow.steps)) return null
  const names = (value: unknown) => Array.isArray(value) ? value.filter((name): name is string => typeof name === 'string' && !!name) : []
  const assets = file.assets
  return {
    name: typeof workflow.name === 'string' ? workflow.name : '',
    agents: names(workflow.agents),
    channels: names(workflow.channels),
    assets: assets && typeof assets === 'object' ? Object.keys(assets).length : 0,
  }
}

// MARK: answer fields ("name: type" lines)

export type AnswerFieldType = FlowFieldType

export interface AnswerFieldRow {
  name: string
  type: AnswerFieldType
  values: string[]
  optional: boolean
}

export interface ParsedAnswerSpec {
  rows: AnswerFieldRow[]
  /** Lines the rows cannot show; kept in the old text box so nothing is lost on a save. */
  unknown: string
}

const ANSWER_TYPES: Record<string, AnswerFieldType> = {
  text: 'text', string: 'text',
  number: 'number', float: 'number',
  int: 'integer', integer: 'integer',
  'yes/no': 'yesNo', yesno: 'yesNo', boolean: 'yesNo',
  list: 'list', array: 'list',
  'one of': 'choice', choice: 'choice',
}

const ANSWER_TYPE_TEXT: Record<AnswerFieldType, string> = {
  text: 'text', number: 'number', integer: 'int', yesNo: 'yes/no', list: 'list', choice: 'one of',
}

/** One `name: type` line per field, separated by newlines or semicolons, as the host saves it. */
export function parseAnswerSpec(text: string): ParsedAnswerSpec {
  const rows: AnswerFieldRow[] = []
  const unknown: string[] = []
  for (const line of text.split(/[\n;]/)) {
    const raw = line.trim()
    if (!raw) continue
    const colon = raw.indexOf(':')
    if (colon < 0) {
      unknown.push(raw)
      continue
    }
    let name = raw.slice(0, colon).trim()
    const typeText = raw.slice(colon + 1).trim()
    const optional = name.endsWith('?')
    if (optional) name = name.slice(0, -1).trim()
    if (!FIELD_NAME.test(name)) {
      unknown.push(raw)
      continue
    }
    const oneOf = /^one of\s+(.+)$/i.exec(typeText)
    const type = ANSWER_TYPES[(oneOf ? 'one of' : typeText).toLocaleLowerCase()]
    const values = oneOf ? oneOf[1].split(',').map((value) => value.trim()).filter(Boolean) : []
    if (!type || (type === 'choice' && values.length < 2)) {
      unknown.push(raw)
      continue
    }
    rows.push({ name, type, values, optional })
  }
  return { rows, unknown: unknown.join('\n') }
}

/** Saved answer text in the host's `name: type` format; a choice keeps its values on the same line. */
export function formatAnswerSpec(rows: readonly AnswerFieldRow[]): string {
  return rows.map((row) => {
    const type = row.type === 'choice' ? `${ANSWER_TYPE_TEXT.choice} ${row.values.join(', ')}` : ANSWER_TYPE_TEXT[row.type]
    return `${row.name}${row.optional ? '?' : ''}: ${type}`
  }).join('\n')
}

// MARK: answer-field results (OpenBot 0.77.0)

/** One field of a step's typed answer, ready for its own row. */
export interface AnswerResultRow {
  name: string
  /** The value as a person reads it: real line breaks, no raw JSON escapes. */
  value: string
}

/**
 * Turns a step's answer-field JSON into one row per field. A JSON `\n` in a value becomes a real
 * line break; a literal backslash-n stays text. A plain text answer gives no rows, so the caller
 * keeps showing it as text. "Copy result" copies the raw output, never these rows.
 */
export function answerResultRows(output: string | null): AnswerResultRow[] {
  if (!output?.trim()) return []
  let parsed: unknown
  try {
    parsed = JSON.parse(output)
  } catch {
    return []
  }
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return []
  const rows: AnswerResultRow[] = []
  for (const [name, value] of Object.entries(parsed)) {
    if (value === null || value === undefined) continue
    if (Array.isArray(value)) rows.push({ name, value: value.map(String).join(', ') })
    else if (typeof value === 'boolean') rows.push({ name, value: value ? 'yes' : 'no' })
    else if (typeof value === 'object') rows.push({ name, value: JSON.stringify(value) })
    else rows.push({ name, value: String(value) })
  }
  return rows
}

// MARK: node registry (values the host knows)

export interface FlowSettingSpec {
  /** The values this setting takes; absent means any value is fine. */
  values?: string[]
  /** The value is a list of those values (comma-separated or a JSON array). */
  list?: boolean
}

/** Which setting values this host knows, by step kind then setting name. */
export type FlowNodeRegistry = Record<string, Record<string, FlowSettingSpec>>

export interface SettingProblem {
  name: string
  /** The allowed values, for a value the host doesn't know. Empty when the name itself is newer. */
  values: string[]
  /** The setting name is missing from this host's registry: it came from a newer version. */
  newer: boolean
}

function settingList(value: unknown): string[] {
  if (Array.isArray(value)) return value.map(String)
  if (typeof value !== 'string') return []
  const text = value.trim()
  if (text.startsWith('[')) {
    try {
      const parsed: unknown = JSON.parse(text)
      return Array.isArray(parsed) ? parsed.map(String) : []
    } catch {
      return []
    }
  }
  return text.split(',').map((item) => item.trim()).filter(Boolean)
}

/** Settings this host doesn't understand: a refused value (with the allowed ones), or a newer field. */
export function unknownSettings(
  kind: string,
  settings: Record<string, unknown>,
  registry: FlowNodeRegistry | null | undefined,
): SettingProblem[] {
  const spec = registry?.[kind] ?? registry?.['*']
  if (!spec) return []
  const problems: SettingProblem[] = []
  for (const [name, value] of Object.entries(settings)) {
    if (value === undefined || value === null || value === '') continue
    const field = spec[name]
    if (!field) {
      problems.push({ name, values: [], newer: true })
      continue
    }
    if (!field.values?.length) continue
    const values = field.list ? settingList(value) : [String(value)]
    if (values.some((item) => !field.values!.includes(item))) problems.push({ name, values: field.values, newer: false })
  }
  return problems
}

/** A step setting with a name this host has never seen means the workflow came from a newer version. */
export function isNewerSetting(
  kind: string,
  settings: Record<string, unknown>,
  registry: FlowNodeRegistry | null | undefined,
): boolean {
  return unknownSettings(kind, settings, registry).some((problem) => problem.newer)
}

/** A workflow with a setting from a newer version must stay read-only, or a save would change that step. */
export function isFlowReadOnly(canvas: FlowCanvas | null, registry: FlowNodeRegistry | null | undefined = canvas?.nodeRegistry): boolean {
  if (!canvas || canvas.readOnly) return !!canvas?.readOnly
  if ((canvas.formatVersion ?? FLOW_FORMAT_VERSION) > FLOW_FORMAT_VERSION) return true
  if (!registry) return false
  for (const link of canvas.links) {
    if (isNewerSetting('bot', link.modelOptions ?? {}, registry)) return true
  }
  for (const media of canvas.mediaNodes) {
    const kind = media.kind === 'video' ? 'video' : 'image'
    const settings: Record<string, unknown> = {
      model: media.model, prompt: media.prompt, aspect: media.aspect, duration: media.duration,
      resolution: media.resolution, transparent: media.transparent, voice: media.voice,
    }
    if (isNewerSetting(kind, settings, registry)) return true
  }
  return false
}

// MARK: portable workflow graph (OpenBot 0.70 contracts, `ipc-workflows.ts`)

// The host owns the graph and its rules. These are the client's read-only mirror of the constants
// and pure decisions a canvas needs to draw the node kinds, refuse an impossible line and label a
// run. Nothing here saves: the view sends the person's choice to the host and reloads the canvas.

export const WORKFLOW_NODE_KINDS = [
  'text', 'prompt', 'asset', 'llm', 'agent', 'image', 'trigger', 'condition', 'approval', 'output', 'note', 'shape',
] as const
export type WorkflowNodeKind = (typeof WORKFLOW_NODE_KINDS)[number]
export const WORKFLOW_ANNOTATION_KINDS = ['text', 'note', 'shape'] as const
export const WORKFLOW_PORT_TYPES = ['text', 'image', 'file'] as const
export type WorkflowPortType = (typeof WORKFLOW_PORT_TYPES)[number]
export const WORKFLOW_BRANCHES = ['yes', 'no'] as const
export type WorkflowBranch = (typeof WORKFLOW_BRANCHES)[number]
export const WORKFLOW_TRIGGER_TYPES = [
  'manual', 'schedule', 'webhook', 'message', 'github', 'discord', 'telegram', 'slack',
] as const
export type WorkflowTriggerType = (typeof WORKFLOW_TRIGGER_TYPES)[number]
export const WORKFLOW_OUTPUT_TARGETS = [
  'notify', 'channel', 'file', 'github', 'discord', 'telegram', 'slack', 'chat-channel',
] as const
export const WORKFLOW_GITHUB_EVENTS = [
  'issue.opened', 'issue.closed', 'issue.labeled', 'pr.opened', 'pr.updated', 'pr.reviewRequested',
  'pr.reviewSubmitted', 'pr.checksFailed', 'pr.merged', 'comment', 'mention', 'assigned',
] as const
export const WORKFLOW_GITHUB_FILTERS = ['notBot', 'firstTime', 'title', 'branch'] as const
export const WORKFLOW_GITHUB_ACTIONS = [
  'comment', 'label.add', 'label.remove', 'assign', 'review.request', 'review.approve',
  'review.changes', 'close', 'reopen', 'issue.create',
] as const
export const WORKFLOW_GITHUB_CLOSE_REASONS = ['completed', 'notPlanned'] as const
export const WORKFLOW_CHAT_EVENTS = ['message', 'mention', 'reaction', 'join'] as const
export const WORKFLOW_CHAT_ACTIONS = ['reply', 'post', 'react', 'label', 'assign', 'close'] as const
export const WORKFLOW_CHAT_EMOJI = ['thumbsup', 'heart', 'fire', 'eyes', 'tada'] as const
export const WORKFLOW_IMAGE_MODES = ['single', 'frames', 'references', 'videos', 'audios'] as const
export const WORKFLOW_IMAGE_INPUTS = ['frames', 'references', 'videos', 'audios'] as const
export const WORKFLOW_RUN_SOURCES = [
  'manual', 'schedule', 'webhook', 'github', 'discord', 'telegram', 'slack',
] as const
export const WORKFLOW_VERSION_KINDS = ['auto', 'restore', 'run'] as const
export const WORKFLOW_DELIVERY_OUTCOMES = ['started', 'waiting', 'skipped'] as const
export const WORKFLOW_SKIP_REASONS = ['filter', 'self', 'chain', 'offline', 'refused'] as const
export const WORKFLOW_MAX_VERSIONS = 50

export type WorkflowFieldValue = string | number | boolean
export type WorkflowFields = Record<string, WorkflowFieldValue>

export interface WorkflowPoint { x: number; y: number }
export interface WorkflowNodeSize { width?: number; height?: number; font?: number }
export interface WorkflowNode {
  id: string
  kind: WorkflowNodeKind
  at: WorkflowPoint
  size?: WorkflowNodeSize
  /** A locked or unknown node is shown read-only, never saved by this client. */
  locked?: boolean
  fields: WorkflowFields
  schedule?: unknown
}
export interface WorkflowEdge {
  id: string
  from: string
  to: string
  type: WorkflowPortType
  branch?: WorkflowBranch
}
export interface WorkflowStroke {
  id: string
  brush: string
  color: string
  width: number
  points: WorkflowPoint[]
}
export interface WorkflowGraph {
  nodes: WorkflowNode[]
  edges: WorkflowEdge[]
  strokes: WorkflowStroke[]
}
export interface WorkflowTriggerState {
  nodeId: string
  type: WorkflowTriggerType
  nextDueAt?: string | null
  webhookUrl?: string | null
}
export interface WorkflowVersion {
  id: string
  at: string
  kind: (typeof WORKFLOW_VERSION_KINDS)[number]
  revision: number
}
export interface WorkflowRun {
  id: string
  workflowId: string
  number: number
  triggerNodeId: string | null
  source: (typeof WORKFLOW_RUN_SOURCES)[number]
  status: string
  error: string | null
  startedAt: string
  endedAt: string | null
}
export interface WorkflowHistoryStep {
  id: string
  workflowId: string
  runId: string | null
  nodeId: string
  attempt: number
  agentId: string | null
  status: string
  output: string | null
  branch: WorkflowBranch | null
  error: string | null
  startedAt: string | null
  endedAt: string | null
  waiting?: boolean
}
export interface WorkflowRunDetail { run: WorkflowRun; steps: WorkflowHistoryStep[] }
export interface WorkflowRunPage {
  runs: WorkflowRunDetail[]
  soloSteps: WorkflowHistoryStep[]
  nextBefore: string | null
}
export interface WorkflowStepSource {
  from: string
  type: WorkflowPortType
  stepId: string | null
  attachmentId: string | null
}
export interface WorkflowStepSettingsAnswer {
  settings: WorkflowFields | null
  sources: WorkflowStepSource[] | null
}
export interface WorkflowDelivery {
  id: string
  nodeId: string
  event: string
  item: { repository: string; number: number; pullRequest: boolean; title: string } | null
  receivedAt: string
  outcome: (typeof WORKFLOW_DELIVERY_OUTCOMES)[number]
  reason: (typeof WORKFLOW_SKIP_REASONS)[number] | null
  runId: string | null
}
export interface WorkflowGitHubHistory {
  deliveries: WorkflowDelivery[]
  runs: { runId: string; item: NonNullable<WorkflowDelivery['item']> }[]
}

/** Which port types each node kind accepts and gives, from OpenBot's `WORKFLOW_PORTS`. */
export const WORKFLOW_PORTS: Record<WorkflowNodeKind, { in: readonly WorkflowPortType[]; out: readonly WorkflowPortType[] }> = {
  trigger: { in: [], out: ['text'] },
  prompt: { in: [], out: ['text'] },
  asset: { in: [], out: WORKFLOW_PORT_TYPES },
  llm: { in: WORKFLOW_PORT_TYPES, out: ['text'] },
  agent: { in: WORKFLOW_PORT_TYPES, out: ['text'] },
  image: { in: WORKFLOW_PORT_TYPES, out: ['image', 'file'] },
  condition: { in: ['text'], out: ['text'] },
  approval: { in: ['text'], out: ['text'] },
  output: { in: WORKFLOW_PORT_TYPES, out: [] },
  text: { in: [], out: [] },
  note: { in: [], out: [] },
  shape: { in: [], out: [] },
}

export const isAnnotationKind = (kind: string): boolean => (WORKFLOW_ANNOTATION_KINDS as readonly string[]).includes(kind)
export const isWorkflowNodeKind = (kind: string): kind is WorkflowNodeKind =>
  (WORKFLOW_NODE_KINDS as readonly string[]).includes(kind)

/** Why a line can't be drawn, in the host's words; null when it can (OpenBot `workflowEdgeProblem`). */
export function graphEdgeProblem(
  fromKind: string,
  toKind: string,
  type: WorkflowPortType,
  branch?: WorkflowBranch,
): 'annotation' | 'port' | 'condition-branch' | 'branch' | null {
  if (!isWorkflowNodeKind(fromKind) || !isWorkflowNodeKind(toKind)) return 'port'
  if (isAnnotationKind(fromKind) || isAnnotationKind(toKind)) return 'annotation'
  if (!WORKFLOW_PORTS[fromKind].out.includes(type) || !WORKFLOW_PORTS[toKind].in.includes(type)) return 'port'
  if (fromKind === 'condition') return branch === undefined ? 'condition-branch' : null
  return branch === undefined ? null : 'branch'
}

/** The problem text shown on an edge or in the note before a refused drop. */
export const GRAPH_PROBLEM_TEXT: Record<string, string> = {
  annotation: 'Notes and shapes cannot be connected.',
  port: 'That port cannot carry this kind of file.',
  branch: 'Only a condition has yes and no outputs.',
  'condition-branch': 'Choose the yes or no output of this condition.',
  duplicate: 'These steps are already connected that way.',
}

/** A node this client may edit: a known kind that isn't locked by the document. */
export function nodeEditable(node: WorkflowNode, needsNewerApp = false): boolean {
  return !needsNewerApp && !node.locked && isWorkflowNodeKind(node.kind)
}

/** The one-line title of a portable node, from its fields (the name the person gave wins). */
export function nodeTitle(node: WorkflowNode): string {
  const fields = node.fields
  const named = (key: string) => {
    const value = fields[key]
    return typeof value === 'string' ? value.trim() : ''
  }
  const own = named('stepName') || named('question') || named('task') || named('prompt') || named('text')
  if (own) return own
  switch (node.kind) {
    case 'llm': return named('model') ? `Model · ${named('model')}` : 'Model step'
    case 'condition': return 'Condition'
    case 'approval': return 'Approval'
    case 'output': return named('action') ? `Output · ${named('action')}` : 'Output'
    case 'asset': return 'File'
    case 'trigger': return named('trigger') ? `Trigger · ${named('trigger')}` : 'Trigger'
    case 'image': return 'Picture or clip'
    case 'prompt': return 'Prompt'
    case 'note': return 'Note'
    case 'shape': return 'Shape'
    case 'agent': return named('agentId') || 'Agent'
    default: return node.kind
  }
}

/** The SF Symbol of a portable node kind, so a card reads before its title does. */
export const NODE_ICON: Record<WorkflowNodeKind, string> = {
  text: 'text.alignleft', prompt: 'text.bubble', asset: 'doc', llm: 'brain', agent: 'sparkles',
  image: 'photo', trigger: 'bolt', condition: 'questionmark.circle', approval: 'hand.raised',
  output: 'square.and.arrow.up', note: 'note.text', shape: 'square.dashed',
}

/**
 * The fields a trigger gives to later steps, mirroring the host's `trigger_fields` projection and
 * OpenBot's `workflowTriggerFields`. Names and types must match what the host freezes on a run, so
 * a `{{name}}` chip resolves without a model step before it.
 */
export function triggerFields(kind: string): FlowField[] {
  const field = (name: string, type: FlowFieldType = 'text'): FlowField => ({ name, step: '', icon: 'bolt', type, values: [] })
  if (kind === 'schedule') return ['run_time', 'local_date', 'local_time', 'weekday'].map((name) => field(name))
  if (kind === 'github') return [
    'repository', 'actor', 'action', 'number', 'pull_request', 'url', 'title', 'body', 'author', 'state',
    'labels', 'label', 'assignee', 'reviewer', 'comment_author', 'comment_body', 'comment_url',
    'review_author', 'review_state', 'review_body', 'review_url', 'check_conclusion', 'check_app', 'head_branch',
  ].map((name) => field(name, name === 'number' ? 'integer' : name === 'pull_request' ? 'yesNo' : name === 'labels' ? 'list' : 'text'))
  const message = ['text', 'author', 'author_id', 'message_id'].map((name) => field(name))
  if (kind === 'discord') return [...message, ...['channel', 'channel_id', 'server'].map((name) => field(name))]
  if (kind === 'telegram') return [...message, ...['chat', 'chat_id'].map((name) => field(name))]
  if (kind === 'slack') return [...message, ...['channel', 'channel_id', 'workspace', 'thread_id'].map((name) => field(name))]
  return []
}

/** The SVG path of a freehand stroke, in canvas coordinates (OpenBot's drawing tool). */
export function strokePath(stroke: WorkflowStroke): string {
  const points = stroke.points
  if (points.length === 0) return ''
  if (points.length === 1) {
    const { x, y } = points[0]
    return `M ${x} ${y} L ${x + 0.01} ${y}`
  }
  return points.map((point, index) => `${index === 0 ? 'M' : 'L'} ${point.x} ${point.y}`).join(' ')
}

/** The words of a version, a run source, a delivery outcome and a skip reason. */
export const VERSION_KIND_LABEL: Record<string, string> = { auto: 'Autosaved', restore: 'Restored', run: 'After a run' }
export const RUN_SOURCE_LABEL: Record<string, string> = {
  manual: 'Manual', schedule: 'Schedule', webhook: 'Webhook', github: 'GitHub',
  discord: 'Discord', telegram: 'Telegram', slack: 'Slack',
}
export const DELIVERY_OUTCOME_LABEL: Record<string, string> = { started: 'Started a run', waiting: 'Waiting', skipped: 'Skipped' }
export const SKIP_REASON_LABEL: Record<string, string> = {
  filter: 'Did not match the filters', self: 'Sent by the bot', chain: 'Part of the same chain',
  offline: 'This computer was offline', refused: 'The host refused it',
}
export const labelOf = (table: Record<string, string>, value: string | null | undefined, tr: (text: string) => string): string =>
  tr(value ? table[value] ?? value : 'Not recorded')

/** `#12 · Manual · 3 steps`, the one line a paginated run row shows. */
export function runSummary(run: WorkflowRun, steps: number, tr: (text: string) => string): string {
  return `#${run.number} · ${labelOf(RUN_SOURCE_LABEL, run.source, tr)} · ${tr('{count} steps').replace('{count}', String(steps))}`
}

/** The attempt, branch and waiting marks a history step carries beside its result. */
export function stepMarks(step: WorkflowHistoryStep): { attempt?: string; branch?: WorkflowBranch; waiting: boolean } {
  return {
    attempt: step.attempt > 1 ? `#${step.attempt}` : undefined,
    branch: step.branch ?? undefined,
    waiting: step.waiting === true,
  }
}

/** Which page cursor a "Load older runs" button sends next; null ends the list. */
export const nextRunCursor = (page: WorkflowRunPage | null): string | null => page?.nextBefore ?? null

/** A portable history status in the words the canvas's status chip already knows. */
export function historyStatus(status: string): StepStatus {
  const known: Record<string, StepStatus> = {
    running: 'running', approval: 'queued', stopping: 'running', done: 'succeeded',
    failed: 'failed', stopped: 'cancelled', skipped: 'skipped',
  }
  return known[status] ?? 'queued'
}

/** A portable node's canvas key, namespaced so it can't collide with a bot or a media step. */
export const workflowKey = (id: string) => `workflow:${id}`
export const workflowIdOf = (key: string) => key.startsWith('workflow:') ? key.slice('workflow:'.length) : key

/** The portable nodes as canvas nodes, at their saved places (or the origin). */
export function graphFlowNodes(graph: WorkflowGraph | null): FlowNode[] {
  return (graph?.nodes ?? []).map((node) => ({
    key: workflowKey(node.id),
    kind: 'workflow' as const,
    id: node.id,
    x: node.at?.x ?? 0,
    y: node.at?.y ?? 0,
    saved: true,
  }))
}

/** A portable edge drawn on the canvas, with the port, branch and host problem it carries. */
export interface GraphFlowEdge extends FlowEdge {
  type?: WorkflowPortType
  branch?: WorkflowBranch
  problem?: string | null
}

/** The portable edges as canvas edges, with the port type and branch the line carries. */
export function graphFlowEdges(graph: WorkflowGraph | null, problems: Record<string, string> = {}): GraphFlowEdge[] {
  return (graph?.edges ?? []).map((edge) => ({
    id: edge.id,
    from: workflowKey(edge.from),
    to: workflowKey(edge.to),
    linkId: edge.id,
    type: edge.type,
    branch: edge.branch,
    problem: problems[edge.id] ?? null,
  }))
}
