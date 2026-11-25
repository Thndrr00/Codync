import type { RosterViewFilter } from './roster-view-filter.ts'

/**
 * The unread tallies the roster's tabs read (OpenBot 0.70.0's All / Unread / folders tabs). Counts
 * are messages, not bots, so a tab's badge says how much waiting there is. The folder names are whole
 * section paths; `''` is the rows without a section.
 */
export interface RosterTabCounts {
  all: number
  unread: number
  folders: Readonly<Record<string, number>>
}

export interface RosterTab {
  id: string
  label: string
  badge: number
}

export interface RosterTabView {
  tabs: RosterTab[]
  /** The id selected by the current view filter (a single folder selects that folder's tab). */
  selected: string
}

/**
 * Pure decision for the tab strip: All first, Unread next, then folders by name. A folder with no
 * unread still gets a tab so it can be inspected; its badge is hidden. The badge on All is the whole
 * roster's unread, so it stays visible under any tab.
 */
export function rosterTab(counts: RosterTabCounts, filter?: RosterViewFilter): RosterTabView {
  const tabs: RosterTab[] = [
    { id: 'all', label: 'All', badge: Math.max(0, counts.all) },
    { id: 'unread', label: 'Unread', badge: Math.max(0, counts.unread) },
  ]
  for (const folder of Object.keys(counts.folders).sort((a, b) => a.localeCompare(b))) {
    tabs.push({ id: folder, label: folder ? folder.split('/').filter(Boolean).at(-1) ?? folder : 'Bots', badge: Math.max(0, counts.folders[folder] ?? 0) })
  }
  const selected = filter?.unread ? 'unread' : filter?.sections.length === 1 ? filter.sections[0]! : 'all'
  return { tabs, selected: tabs.some((tab) => tab.id === selected) ? selected : 'all' }
}

/**
 * A tab click only changes the unread and section parts of the view filter: the person's kind,
 * states, sort and other folder choices are kept, so the strip and the filter menu agree.
 */
export function selectRosterTab(filter: RosterViewFilter, id: string): RosterViewFilter {
  if (id === 'unread') return { ...filter, unread: true, sections: [] }
  if (id === 'all') return { ...filter, unread: false, sections: [] }
  return { ...filter, unread: false, sections: [id] }
}
