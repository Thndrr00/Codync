// The host's field-level flow merge (host/src/routines/flow/merge.rs). A document is the four
// drawing fields keyed by id; a save carries the revision the person started from. The host folds
// the two edits field by field and, when both changed one entry differently, asks which version
// to keep. The other fields still use their own granular methods and are only snapshotted here.

import type { FlowCanvas } from '../../../lib/routine-flow.ts'
import type { FlowGroup } from './flow-groups'

export type FlowEntries = Record<string, Record<string, unknown>>

/** One bot's flow document: nodes, links, step settings and groups, each keyed by id. */
export interface FlowDocument {
  rev: number
  nodes: FlowEntries
  links: FlowEntries
  settings: FlowEntries
  groups: FlowEntries
}

/** One entry both people changed differently; a missing version means that side removed it. */
export interface FlowConflict {
  field: string
  key: string
  local: Record<string, unknown> | null
  incoming: Record<string, unknown> | null
}

export interface FlowSaveAnswer {
  saved?: FlowDocument
  document: FlowDocument
  conflicts?: FlowConflict[]
  merged: boolean
}

export type FlowChoice = 'local' | 'incoming'

/** The canvas's drawing state as a document, with `groups` replaced by the list being saved. */
export function documentFromCanvas(canvas: FlowCanvas, groups: FlowGroup[]): FlowDocument {
  const nodes: FlowEntries = {}
  for (const position of canvas.positions) nodes[position.nodeKey] = { x: position.x, y: position.y }
  for (const id of canvas.placedBotIds) nodes[`bot:${id}`] = { ...nodes[`bot:${id}`], placed: true }
  const links: FlowEntries = {}
  for (const link of canvas.links) links[link.id] = { ...link }
  const settings: FlowEntries = {}
  for (const media of canvas.mediaNodes) settings[media.id] = { ...media }
  const byGroup: FlowEntries = {}
  for (const group of groups) byGroup[group.id] = { ...group }
  return { rev: canvas.rev, nodes, links, settings, groups: byGroup }
}
