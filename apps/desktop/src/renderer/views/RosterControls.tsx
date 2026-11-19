import { useEffect, useMemo, useRef, useState } from 'react'
import { ChoicePicker, Button, DropdownMenu, IconButton } from '../components/Controls'
import { Icon } from '../components/Icon'
import type { MenuItem } from '../components/Menu'
import { ModalHeader, Sheet } from '../components/Overlay'
import { HStack, VStack } from '../components/Stack'
import { Text } from '../components/Text'
import { useModel } from '../lib/observable'
import type { RosterGrouping } from '../lib/roster-query'
import {
  NO_ROSTER_VIEW_FILTER, rosterSectionMark, rosterSectionTree, rosterViewFilterIsSet, toggleRosterSection,
  type RosterSectionNode, type RosterViewFilter,
} from '../lib/roster-view-filter'
import { useTr } from '../lib/use-language'
import type { RosterItem } from '../store/app-model'
import { RosterQuery, openRosterHit } from '../store/roster-query'
import './roster-controls.css'
import { SavedHistorySearches } from './SavedHistorySearches'
import type { SearchScope } from '../store/search-shortcuts'

export function RosterControls({ query, setQuery, viewFilter, setViewFilter, grouping, setGrouping, active, stopAll, allItems }: {
  query: string; setQuery: (value: string) => void
  viewFilter: RosterViewFilter; setViewFilter: (value: RosterViewFilter) => void
  grouping: RosterGrouping; setGrouping: (value: RosterGrouping) => void
  /** Bots working or waiting on the user, across the computers shown. */
  active: number; stopAll: () => void
  /** Every bot the roster can show, for the filter's sections. */
  allItems: RosterItem[]
}) {
  return <VStack gap={8} className="roster-controls">
    <HStack gap={6} className="roster-search-row">
      <input className="roster-search" aria-label="Filter bots by name, project or agent" placeholder="Name, project or agent" value={query} onChange={(event) => setQuery(event.target.value)} />
      <RosterFilterButton filter={viewFilter} onChange={setViewFilter} items={allItems} />
    </HStack>
    <HStack gap={8}>
      <ChoicePicker selection={grouping} options={[{ id: 'manual', label: 'Manual order' }, { id: 'project', label: 'By project' }]} onChange={setGrouping} />
      {active ? <IconButton title={`Stop all (${active})`} icon="stop.fill" onClick={stopAll} /> : null}
    </HStack>
  </VStack>
}

/** The filter beside the search (OpenBot 0.73.0's sidebar filter): a dot shows that one is set. */
function RosterFilterButton({ filter, onChange, items }: { filter: RosterViewFilter; onChange: (value: RosterViewFilter) => void; items: RosterItem[] }) {
  const tr = useTr()
  const sectionKey = items.map((item) => item.bot.section ?? '').join('\u0000')
  const roots = useMemo(() => rosterSectionTree(items.map((item) => item.bot)), [sectionKey])
  const set = rosterViewFilterIsSet(filter)
  const label = set ? tr('Filter bots, a filter is set') : tr('Filter bots')
  return <DropdownMenu
    className={`roster-filter-button${set ? ' set' : ''}`}
    style={{ width: 38, height: 38, borderRadius: 12 }}
    title={label}
    label={label}
    live
    items={() => rosterFilterItems(filter, onChange, roots)}
  >
    <Icon name="slider.horizontal.3" size={13} weight="medium" />
    {set ? <span className="roster-filter-dot" aria-hidden /> : null}
  </DropdownMenu>
}

/** The filter's rows: the kind, the states, the sections (nested ones open their own menu) and the sort. */
function rosterFilterItems(filter: RosterViewFilter, onChange: (value: RosterViewFilter) => void, roots: RosterSectionNode[]): MenuItem[] {
  const set = (patch: Partial<RosterViewFilter>) => onChange({ ...filter, ...patch })
  const rows: MenuItem[] = [
    { title: 'All', selected: filter.show === 'all', keepsOpen: true, action: () => set({ show: 'all' }) },
    { title: 'Agents', selected: filter.show === 'agents', keepsOpen: true, action: () => set({ show: 'agents' }) },
    { title: 'Channels', selected: filter.show === 'channels', keepsOpen: true, action: () => set({ show: 'channels' }) },
    { title: 'Only with unread', divider: true, selected: filter.unread, keepsOpen: true, action: () => set({ unread: !filter.unread }) },
    { title: 'Only working', selected: filter.working, keepsOpen: true, action: () => set({ working: !filter.working }) },
    { title: 'Only needs attention', selected: filter.attention, keepsOpen: true, action: () => set({ attention: !filter.attention }) },
    { title: 'Only pinned', selected: filter.pinned, keepsOpen: true, action: () => set({ pinned: !filter.pinned }) },
  ]
  if (roots.length) rows.push({ title: 'Sections', icon: 'folder', divider: true, submenu: sectionRows(roots, filter, set) })
  rows.push({
    title: 'Sort',
    detail: filter.sort === 'recent' ? 'Recent activity' : 'Manual order',
    divider: true,
    submenu: [
      { title: 'Manual order', selected: filter.sort === 'manual', action: () => set({ sort: 'manual' }) },
      { title: 'Recent activity', selected: filter.sort === 'recent', action: () => set({ sort: 'recent' }) },
    ],
  })
  if (rosterViewFilterIsSet(filter)) rows.push({ title: 'Clear filters', icon: 'xmark', divider: true, action: () => onChange(NO_ROSTER_VIEW_FILTER) })
  return rows
}

/** Every section, with "All sections" first; picking one keeps the menu open for the next. */
function sectionRows(roots: RosterSectionNode[], filter: RosterViewFilter, set: (patch: Partial<RosterViewFilter>) => void): MenuItem[] {
  const all: MenuItem = { title: 'All sections', selected: filter.sections.length === 0, keepsOpen: true, action: () => set({ sections: [] }) }
  return [all, ...roots.map((node, index) => ({ ...sectionRow(node, filter, roots, set), ...(index === 0 ? { divider: true } : {}) }))]
}

function sectionRow(node: RosterSectionNode, filter: RosterViewFilter, roots: RosterSectionNode[], set: (patch: Partial<RosterViewFilter>) => void): MenuItem {
  const mark = rosterSectionMark(node, filter.sections)
  const toggle = () => set({ sections: toggleRosterSection(node, roots, filter.sections) })
  if (!node.children.length) {
    return { title: node.label, verbatim: node.name !== '', selected: mark === 'on', keepsOpen: true, action: toggle }
  }
  return {
    title: node.label,
    verbatim: node.name !== '',
    leading: mark === 'off' ? undefined : <Icon name={mark === 'on' ? 'checkmark' : 'minus.circle'} size={10} weight="semibold" />,
    submenu: [
      { title: node.label, verbatim: node.name !== '', selected: mark === 'on', keepsOpen: true, action: toggle },
      ...node.children.map((child) => sectionRow(child, filter, roots, set)),
    ],
  }
}

export function RosterHistory({ items, onPick, onClose }: { items: RosterItem[]; onPick: (item: RosterItem) => void; onClose: () => void }) {
  const model = useMemo(() => new RosterQuery(), [])
  useModel(model)
  const [query, setQuery] = useState('')
  const [scope, setScope] = useState<SearchScope>('all')
  const windows = window.codync.platform === 'win32'
  const stores = [...new Set(items.map((item) => item.store))]
  const [opening, setOpening] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const key = JSON.stringify(items.map((i) => [i.store.contextId, i.ref.computerId, i.bot.id]))
  const request = useRef(0)
  useEffect(() => {
    request.current++
    setOpening(false)
    model.find(query, items, false, scope)
    return () => { request.current++; model.dispose() }
  }, [model, query, key, scope])
  const result = model.result
  return <VStack gap={12} className="roster-history">
    <input autoFocus className="roster-search" value={query} aria-label="Search full message history" placeholder="Search message history" onChange={(e) => setQuery(e.target.value)} />
    {windows ? <SavedHistorySearches stores={stores} query={query} scope={scope} restore={(value, filter) => { setQuery(value); setScope(filter) }} /> : null}
    {items.some((i) => i.store.connection.kind !== 'online') ? <Text tone="attention">Connect to search the computer's full history.</Text> : null}
    {error ? <Text tone="danger">{error}</Text> : null}
    {result.kind === 'idle' ? <Text tone="secondary">Search messages, including threads.</Text> : null}
    {result.kind === 'loading' ? <Text tone="secondary">Searching…</Text> : null}
    {result.kind === 'error' ? <HStack gap={8}><Text tone="danger">{result.message}</Text>{windows ? <Button kind="secondary" onClick={() => model.find(query, items, false, scope)}>Retry search</Button> : null}</HStack> : null}
    {result.kind === 'ready' ? <>
      {windows && result.warning ? <HStack gap={8}><Text tone="attention">{result.warning}</Text><Button kind="secondary" onClick={() => model.find(query, items, false, scope)}>Retry search</Button></HStack> : null}
      {!result.hits.length ? <Text tone="secondary">No matching messages</Text> : null}
      {result.hits.map(({ hit, item }) => <button className="roster-hit" key={`${item.ref.computerId}:${hit.id}`} disabled={opening} onClick={() => {
        setOpening(true)
        const token = ++request.current
        void openRosterHit(item, hit, () => request.current === token).then((opened) => { if (opened) { onPick(item); onClose() } }).catch((e: unknown) => {
          if (request.current !== token) return
          setError(e instanceof Error ? e.message : String(e)); setOpening(false)
        })
      }}><Text as="div" weight="semibold">{item.bot.name} · {hit.threadId ? 'Thread' : 'Main chat'}</Text><Text as="div" tone="secondary">{hit.text}</Text></button>)}
      {[...result.cursors.values()].some((v) => v !== null) ? <Button kind="secondary" onClick={() => model.find(query, items, true, scope)}>More messages</Button> : null}
    </> : null}
  </VStack>
}

export function RosterPreview({ item, close, open }: { item: RosterItem; close: () => void; open: () => void }) {
  const model = useMemo(() => new RosterQuery(), [item.store, item.bot.id])
  useModel(model)
  useEffect(() => { void model.showPreview(item); return () => model.dispose() }, [model])
  return <Sheet open onClose={close} width={480} dismissOnDim>
    <ModalHeader title={item.bot.name} subtitle="Main chat preview · unread unchanged" onClose={close} />
    <VStack gap={12} className="roster-history">
      {model.previewError ? <Text tone="attention">{model.previewError}</Text> : model.preview === null ? <Text tone="secondary">Loading preview…</Text>
        : model.preview.length === 0 ? <Text tone="secondary">No messages yet</Text>
        : model.preview.map((entry) => <Text as="div" key={entry.id} className="roster-preview-message">{entry.data.text}</Text>)}
      <Button kind="secondary" onClick={open}>Open chat</Button>
    </VStack>
  </Sheet>
}
