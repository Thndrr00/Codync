// Drawing-only groups of a routine-flow canvas (host/src/routines/flow/groups.rs). A group changes
// nothing about what a routine runs: it bundles nodes on the canvas, gives them one port per
// output kind, collapses member→one lines into a counted line, and folds into a stack card.
// The group list is saved in the canvas from the host, so an older host/canvas simply has none.

import {
  flowEdges, layoutNodes, mediaOf, NODE_WIDTH,
  type FlowCanvas, type FlowEdge, type FlowNode,
} from '../../../lib/routine-flow.ts'
export interface FlowGroup {
  id: string
  name: string
  /** Node keys (`routine:`, `bot:` or `media:`). */
  members: string[]
  collapsed: boolean
  /** Position of the group card / port anchor. */
  x: number
  y: number
}

/** A canvas as loaded; `groups` is optional for an older host (step settings are on FlowCanvas). */
export type CanvasWithGroups = FlowCanvas & { groups?: FlowGroup[] }

export const groupsOf = (canvas: CanvasWithGroups): FlowGroup[] => canvas.groups ?? []
export const groupKey = (id: string) => `group:${id}`
export const portPrefix = 'groupport:'
export const portKey = (id: string, type: string) => `${portPrefix}${id}:${type}`

/** A port key's group id and output type, or null for an ordinary key. */
export function parsePort(key: string): { id: string; type: string } | null {
  if (!key.startsWith(portPrefix)) return null
  const rest = key.slice(portPrefix.length)
  const at = rest.indexOf(':')
  return at < 0 ? null : { id: rest.slice(0, at), type: rest.slice(at + 1) }
}

/** The group a node belongs to, if any. */
export function groupOf(groups: readonly FlowGroup[], nodeKey: string): FlowGroup | null {
  return groups.find((group) => group.members.includes(nodeKey)) ?? null
}

/** The output kind a node hands on: a picture, a clip, or a bot's text reply. */
export function outputType(canvas: FlowCanvas, nodeKey: string): string {
  const media = mediaOf(canvas, nodeKey)
  if (media) return media.kind
  return 'text'
}

/** The distinct output kinds of a group's members, in a stable order. */
export function groupPorts(canvas: FlowCanvas, group: FlowGroup): string[] {
  return [...new Set(group.members.map((key) => outputType(canvas, key)))].sort()
}

/** Where the ports of a group sit, to the right of its card. */
export function portPosition(group: FlowGroup, index: number) {
  return { x: group.x + NODE_WIDTH + 44, y: group.y + index * 44 }
}

/**
 * The nodes the canvas draws. A collapsed group hides its members and stands in as one node; an
 * open group keeps its members and adds a node per output port. All keep the `FlowNode` shape so
 * the existing edge, hit-test and fit helpers keep working.
 */
export function groupedNodes(canvas: CanvasWithGroups): FlowNode[] {
  const base = layoutNodes(canvas)
  const hidden = new Set<string>()
  const nodes: FlowNode[] = []
  for (const group of groupsOf(canvas)) {
    if (group.collapsed) {
      for (const key of group.members) hidden.add(key)
      nodes.push({ key: groupKey(group.id), kind: 'bot', id: group.id, x: group.x, y: group.y, saved: true })
    }
    // Both open and folded groups keep one port per output kind, so lines always have an anchor.
    groupPorts(canvas, group).forEach((type, index) => {
      const at = portPosition(group, index)
      const key = portKey(group.id, type)
      nodes.push({ key, kind: 'bot', id: key, x: at.x, y: at.y, saved: true })
    })
  }
  return [...base.filter((node) => !hidden.has(node.key)), ...nodes]
}

/** The group whose stack card stands in for a hidden member. */
function hiddenGroup(groups: readonly FlowGroup[], key: string): FlowGroup | null {
  const group = groupOf(groups, key)
  return group?.collapsed ? group : null
}

export interface GroupedEdge extends FlowEdge {
  /** How many member lines this one stands for (1 for an ordinary edge). */
  count: number
}

/**
 * The edges the canvas draws. A member's link leaves the group's port for its output kind; links
 * from several members to the same node become one line with a count. Edges into hidden members
 * end at the stack card instead. The routine's fixed start edge is re-routed the same way.
 */
export function groupedEdges(canvas: CanvasWithGroups): GroupedEdge[] {
  const groups = groupsOf(canvas)
  const out: GroupedEdge[] = []
  const counted = new Map<string, GroupedEdge>()
  for (const edge of flowEdges(canvas)) {
    if (edge.linkId === null) {
      const to = hiddenGroup(groups, edge.to)
      const from = hiddenGroup(groups, edge.from)
      out.push({
        id: edge.id,
        from: from ? groupKey(from.id) : edge.from,
        to: to ? groupKey(to.id) : edge.to,
        linkId: null,
        count: 1,
      })
      continue
    }
    const group = groupOf(groups, edge.from)
    const to = hiddenGroup(groups, edge.to)
    const from = group ? portKey(group.id, outputType(canvas, edge.from)) : edge.from
    const target = to ? groupKey(to.id) : edge.to
    const id = `${from}|${target}`
    const seen = counted.get(id)
    if (seen) {
      seen.count += 1
    } else {
      const merged: GroupedEdge = { id, from, to: target, linkId: edge.linkId, count: 1 }
      counted.set(id, merged)
      out.push(merged)
    }
  }
  return out
}

/** The bounding box of a set of nodes, used to place a new group's card. */
export function groupSpot(nodes: readonly FlowNode[], keys: readonly string[]) {
  const chosen = nodes.filter((node) => keys.includes(node.key))
  if (chosen.length === 0) return { x: 0, y: 0 }
  return {
    x: Math.min(...chosen.map((node) => node.x)),
    y: Math.min(...chosen.map((node) => node.y)),
  }
}

/** A fresh id for a group, local to the canvas and stable until it is saved. */
export function newGroupId(groups: readonly FlowGroup[]): string {
  let id = 'g1'
  let n = 1
  const used = new Set(groups.map((group) => group.id))
  while (used.has(id)) id = `g${++n}`
  return id
}
