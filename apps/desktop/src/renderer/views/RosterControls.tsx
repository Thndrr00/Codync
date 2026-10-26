import { useEffect, useMemo, useRef, useState } from 'react'
import { ChoicePicker, Button } from '../components/Controls'
import { ModalHeader, Sheet } from '../components/Overlay'
import { HStack, VStack } from '../components/Stack'
import { Text } from '../components/Text'
import { useModel } from '../lib/observable'
import { rosterFilters, type RosterFilter, type RosterGrouping } from '../lib/roster-query'
import type { RosterItem } from '../store/app-model'
import { RosterQuery, openRosterHit } from '../store/roster-query'
import './roster-controls.css'
import { SavedHistorySearches } from './SavedHistorySearches'
import type { SearchScope } from '../store/search-shortcuts'

export function RosterControls({ query, setQuery, filter, setFilter, grouping, setGrouping, attention }: {
  query: string; setQuery: (value: string) => void; filter: RosterFilter; setFilter: (value: RosterFilter) => void
  grouping: RosterGrouping; setGrouping: (value: RosterGrouping) => void; attention: number
}) {
  return <VStack gap={8} className="roster-controls">
    <input className="roster-search" aria-label="Filter bots by name, project or agent" placeholder="Name, project or agent" value={query} onChange={(event) => setQuery(event.target.value)} />
    <HStack gap={8}>
      <ChoicePicker selection={filter} options={rosterFilters.map((f) => ({ ...f, label: f.id === 'attention' ? `${f.label} (${attention})` : f.label }))} onChange={setFilter} />
      <ChoicePicker selection={grouping} options={[{ id: 'manual', label: 'Manual order' }, { id: 'project', label: 'By project' }]} onChange={setGrouping} />
    </HStack>
  </VStack>
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
