import { isGroup, type Bot } from '../../shared/models.ts'

/** What the roster shows: every row, only agents, or only channels (group chats). */
export type RosterShow = 'all' | 'agents' | 'channels'

/** The states that need the user's attention, shared by the filter and the recent-activity sort. */
const ATTENTION_STATUSES: readonly string[] = ['needsInput', 'error']
const needsAttention = (status: string) => ATTENTION_STATUSES.includes(status)

/** How the roster orders its rows inside each section. Manual keeps the person's drag order. */
export type RosterSort = 'manual' | 'recent'

/**
 * The roster's own filter, saved on this device under one key (OpenBot 0.73.0's sidebar filter,
 * adapted to Codync's sections). A view of the list only: it never changes the bots or their order.
 */
export interface RosterViewFilter {
  show: RosterShow
  /** The states are independent toggles; with more than one, a row shows when it has either. */
  unread: boolean
  working: boolean
  attention: boolean
  pinned: boolean
  /** The whole names of the sections to include (`''` is the rows without one); none means every section. */
  sections: string[]
  sort: RosterSort
}

export const NO_ROSTER_VIEW_FILTER: RosterViewFilter = {
  show: 'all', unread: false, working: false, attention: false, pinned: false, sections: [], sort: 'manual',
}

/** A saved filter as the roster reads it: an older or partial one keeps what it can, and anything else is the default. */
export function normalizeRosterViewFilter(value: unknown): RosterViewFilter {
  const raw = (typeof value === 'object' && value !== null ? value : {}) as Partial<RosterViewFilter>
  const show = raw.show === 'agents' || raw.show === 'channels' ? raw.show : 'all'
  const sections = Array.isArray(raw.sections) ? [...new Set(raw.sections.filter((name): name is string => typeof name === 'string'))] : []
  return {
    show,
    unread: raw.unread === true,
    working: raw.working === true,
    attention: raw.attention === true,
    pinned: raw.pinned === true,
    sections,
    sort: raw.sort === 'recent' ? 'recent' : 'manual',
  }
}

/** Whether any part of the filter is on: the button's dot and the empty state read it. */
export function rosterViewFilterIsSet(filter: RosterViewFilter): boolean {
  return filter.show !== 'all' || filter.unread || filter.working || filter.attention || filter.pinned || filter.sections.length > 0
}

/** Whether `bot` passes the view filter (the roster keeps the open bot visible beside this). */
export function matchesRosterView(bot: Bot, filter: RosterViewFilter): boolean {
  if (filter.show === 'agents' && isGroup(bot)) return false
  if (filter.show === 'channels' && !isGroup(bot)) return false
  if (filter.unread || filter.working || filter.attention || filter.pinned) {
    const shown = (filter.unread && bot.unread > 0)
      || (filter.working && bot.status === 'working')
      || (filter.attention && needsAttention(bot.status))
      || (filter.pinned && bot.pinned)
    if (!shown) return false
  }
  if (filter.sections.length === 0) return true
  // Picking a section also picks the sections in it (toggleRosterSection), so each name is exact:
  // unchecking one section in a selected parent then hides just that section.
  return filter.sections.includes(bot.section ?? '')
}

/** A row as the sort reads it: its pin, its state and the time of its last message. */
type SortableRosterBot = Pick<Bot, 'pinned' | 'status' | 'lastAt'>

/**
 * The rows of one section in the chosen order. Recent activity keeps pins and bots that need you
 * above the rest, then shows each band by its last message, newest first. Manual keeps the order the
 * host sent. It copies before sorting, so the person's drag order is never changed and switching
 * back restores it.
 */
export function sortRosterRows<T extends { bot: SortableRosterBot }>(rows: readonly T[], sort: RosterSort): T[] {
  if (sort === 'manual') return [...rows]
  return [...rows].sort((a, b) => attentionBand(a.bot) - attentionBand(b.bot) || b.bot.lastAt - a.bot.lastAt)
}

/** Pins first, then bots that need you, then everyone else. */
function attentionBand(bot: SortableRosterBot): number {
  if (bot.pinned) return 0
  if (needsAttention(bot.status)) return 1
  return 2
}

/** A section and the sections nested in it, as the filter menu shows them. */
export interface RosterSectionNode {
  /** The whole section name (`''` for the rows without one). */
  name: string
  /** Its own last level, which the menu shows. */
  label: string
  children: RosterSectionNode[]
}

/**
 * The sections the bots use as a tree, sorted by name, with the unsectioned rows (`''`) at the
 * end. A bot's section is a whole path (`Work/Clients`), so its parents come with it.
 */
export function rosterSectionTree(bots: Iterable<{ section?: string | null }>): RosterSectionNode[] {
  const roots: RosterSectionNode[] = []
  let unsectioned = false
  for (const bot of bots) {
    const name = bot.section?.trim()
    if (!name) {
      unsectioned = true
      continue
    }
    let list = roots
    let path = ''
    for (const part of name.split('/').filter(Boolean)) {
      path = path ? `${path}/${part}` : part
      let node = list.find((candidate) => candidate.name === path)
      if (!node) {
        node = { name: path, label: part, children: [] }
        list.push(node)
      }
      list = node.children
    }
  }
  sortSectionNodes(roots)
  if (unsectioned) roots.push({ name: '', label: 'Bots', children: [] })
  return roots
}

function sortSectionNodes(nodes: RosterSectionNode[]) {
  nodes.sort((a, b) => a.label.localeCompare(b.label))
  for (const node of nodes) sortSectionNodes(node.children)
}

/** Every section name in `node`'s subtree, the node's own included. */
export function rosterSectionIds(node: RosterSectionNode): string[] {
  return [node.name, ...node.children.flatMap(rosterSectionIds)]
}

/** On when the section and every section in it are selected, mixed when only some are. */
export function rosterSectionMark(node: RosterSectionNode, selected: readonly string[]): 'on' | 'off' | 'mixed' {
  const ids = rosterSectionIds(node)
  const count = ids.filter((id) => selected.includes(id)).length
  if (count === 0) return 'off'
  return count === ids.length ? 'on' : 'mixed'
}

/**
 * Selects or clears a section with every section in it. All sections selected is the same as none
 * (every section), and a section the roster no longer has drops out of the selection.
 */
export function toggleRosterSection(node: RosterSectionNode, roots: readonly RosterSectionNode[], selected: readonly string[]): string[] {
  const allIds = roots.flatMap(rosterSectionIds)
  const next = new Set(selected)
  const on = rosterSectionMark(node, selected) === 'on'
  for (const id of rosterSectionIds(node)) {
    if (on) next.delete(id)
    else next.add(id)
  }
  const ordered = allIds.filter((id) => next.has(id))
  return ordered.length === allIds.length ? [] : ordered
}
