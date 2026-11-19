import { computerSections, moveInOrder, orderedComputers } from '../lib/computer-roster'
import { renamedSectionOrder } from '../lib/roster-section-order'
import { prefs, usePref } from '../lib/prefs'
import { useModel } from '../lib/observable'
import { foldsAfterRename, rosterGroups, rosterLines, sameRosterGroup, sectionRename, shownRows, shownSection } from '../lib/roster-sections'
import type { BotStore } from './bot-store'
import { sameRef, type AppModel, type BotRef, type RosterItem } from './app-model'
import { useState } from 'react'
import { matchesRoster, type RosterGrouping } from '../lib/roster-query'
import { NO_ROSTER_VIEW_FILTER, matchesRosterView, normalizeRosterViewFilter, rosterViewFilterIsSet, sortRosterRows, type RosterViewFilter } from '../lib/roster-view-filter'
import type { ThreadInfo } from '@shared/models'

/** Device-local section ordering and folding, matching the iPhone roster. */
export function useComputerRoster(app: AppModel, shown: Set<string>) {
  const [query, setQuery] = useState('')
  const [grouping, setGrouping] = useState<RosterGrouping>('manual')
  const order = useModel(app.computerOrder).ids
  const [collapsed, setCollapsed] = usePref(prefs.collapsedComputers)
  const [sectionOrders, setSectionOrders] = usePref(prefs.rosterSectionOrder)
  const [folded, setFolded] = usePref(prefs.collapsedRosterSections)
  const [savedViewFilter, setSavedViewFilter] = usePref(prefs.rosterViewFilter)
  const viewFilter = normalizeRosterViewFilter(savedViewFilter)
  const computers = orderedComputers(app.computers, order)
  const allItems = app.roster.filter((item) => shown.has(item.ref.computerId))
  // The open bot stays in the roster while a filter hides it, so the chat never loses its row.
  const shownByView = (item: RosterItem) => matchesRosterView(item.bot, viewFilter) || (app.selection !== null && sameRef(item.ref, app.selection))
  const filtered = query.trim() !== '' || rosterViewFilterIsSet(viewFilter)
  const sections = computerSections(computers.filter((c) => shown.has(c.id)), allItems.filter((i) => matchesRoster(i.bot, query, 'all') && shownByView(i)))
    .map((section) => ({ ...section, items: sortRosterRows(section.items, viewFilter.sort), store: app.store(section.computer.id) }))
    .filter((section): section is typeof section & { store: BotStore } => section.store !== null)
    // While filtering, a computer with no matching bot has no heading to show.
    .filter((section) => !filtered || section.items.length > 0)
  const grouped = sections.length > 1
  return {
    sections,
    allItems, query, setQuery, grouping, setGrouping, filtered,
    viewFilter,
    setViewFilter(next: RosterViewFilter) {
      setSavedViewFilter(normalizeRosterViewFilter(next))
    },
    /** Search and the view filter go back to showing every bot. */
    clearFilters() {
      setQuery('')
      setSavedViewFilter(NO_ROSTER_VIEW_FILTER)
    },
    canReorder: !filtered && grouping === 'manual' && viewFilter.sort === 'manual',
    grouped,
    collapsed,
    folded,
    sectionOrders,
    orderSections(computerId: string, names: string[]) {
      setSectionOrders({ ...prefs.rosterSectionOrder.get(), [computerId]: names })
    },
    items: sections.flatMap((section) => section.items),
    /** The bot's live threads as the chat's thread drawer already loaded them, newest first. */
    threadsFor(item: RosterItem): ThreadInfo[] {
      return item.store.threads.get(item.bot.id) ?? []
    },
    /** The rows on screen, top to bottom, for â†‘/â†“. */
    navigable: sections.flatMap((section) => grouped && collapsed.includes(section.computer.id) ? [] : shownRows(rosterLines(section.items, folded, sectionOrders[section.computer.id]))),
    toggle(id: string) {
      setCollapsed(toggled(collapsed, id))
    },
    /** Folds or unfolds roster section `name` on this device. */
    toggleFold(name: string) {
      setFolded(toggled(folded, name))
    },
    /** Renames or removes (null) a section on `store`'s computer; a folded one stays folded under its new name, on this device, unless the host refuses. */
    renameSection(store: BotStore, from: string, to: string | null) {
      const orders = prefs.rosterSectionOrder.get()
      const oldOrder = orders[store.computer.id] ?? []
      const displayed = rosterGroups(store.roster.map((bot) => ({ bot })), [], oldOrder).map((group) => group.name)
      const newOrder = renamedSectionOrder(displayed, from, sectionRename(from, to).to)
      setSectionOrders({ ...orders, [store.computer.id]: newOrder })
      // Read now: a name dialog saves after the render that built its menu.
      const before = prefs.collapsedRosterSections.get()
      const after = foldsAfterRename(before, from, sectionRename(from, to).to)
      setFolded(after)
      // Refused: the folds go back with the bots (one render, no open-then-fold), unless they were toggled while the host answered.
      void store.sections.rename(from, to, () => {
        const current = prefs.rosterSectionOrder.get()
        if (current[store.computer.id] === newOrder) setSectionOrders({ ...current, [store.computer.id]: oldOrder })
        if (prefs.collapsedRosterSections.get() === after) setFolded(before)
      })
    },
    move(id: string, target: string) {
      app.computerOrder.move(id, target, computers.map((c) => c.id))
    },
    /** One place up (-1) or down (+1) among its computer's bots in the same band (lead, pinned, the rest) and, for the rest, the same section. */
    moveBot(ref: BotRef, offset: number) {
      if (filtered || grouping !== 'manual' || viewFilter.sort !== 'manual') return
      const section = sections.find((s) => s.computer.id === ref.computerId)
      const items = section?.items ?? []
      const moved = items.find((item) => item.bot.id === ref.botId)
      const shown = moved && shownSection(moved.bot)
      // A folded section shows no order to change.
      if (!section || !moved || (shown && folded.includes(shown))) return
      const group = items.filter((item) => sameRosterGroup(item.bot, moved.bot))
      const target = group[group.indexOf(moved) + offset]
      if (target) section.store.reorder(moveInOrder(items.map((item) => item.bot.id), ref.botId, target.bot.id))
    },
  }
}

/** `list` with `value` added, or taken out when it's there. */
function toggled(list: string[], value: string) {
  return list.includes(value) ? list.filter((v) => v !== value) : [...list, value]
}
