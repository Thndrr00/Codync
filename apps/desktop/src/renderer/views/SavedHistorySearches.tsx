import { useEffect, useMemo, useState } from 'react'
import { Button, ChoicePicker, IconButton } from '../components/Controls'
import { HStack, VStack } from '../components/Stack'
import { Text } from '../components/Text'
import { useModel } from '../lib/observable'
import { useTr } from '../lib/use-language'
import type { BotStore } from '../store/bot-store'
import { SearchShortcuts, type SearchScope } from '../store/search-shortcuts'

export const searchScopes: { id: SearchScope; label: string }[] = [
  { id: 'all', label: 'All messages' }, { id: 'main', label: 'Main chats' }, { id: 'threads', label: 'Threads' },
]

export function SavedHistorySearches({ stores, query, scope, restore }: {
  stores: BotStore[]; query: string; scope: SearchScope; restore: (query: string, scope: SearchScope) => void
}) {
  const tr = useTr()
  const [selected, setSelected] = useState('')
  const target = stores.find((store) => store.computer.id === selected) ?? stores[0]
  return <VStack gap={8}>
    <Text size="caption" tone="secondary">{tr('Saved searches')}{target ? ` · ${target.hostName}` : ''}</Text>
    {stores.length > 1 ? <ChoicePicker selection={target?.computer.id ?? ''} options={stores.map((store) => ({ id: store.computer.id, label: store.hostName }))} onChange={(id) => { setSelected(id); restore(query, 'all') }} /> : null}
    {target ? <ComputerSearches key={`${target.contextId}:${target.computer.id}`} store={target} query={query} scope={scope} restore={restore} /> : <Text tone="secondary">Connect to a computer to save searches.</Text>}
  </VStack>
}

function ComputerSearches({ store, query, scope, restore }: {
  store: BotStore; query: string; scope: SearchScope; restore: (query: string, scope: SearchScope) => void
}) {
  const model = useModel(useMemo(() => new SearchShortcuts(() => store.ready()), [store]))
  useEffect(() => { model.resume(); void model.load(); return () => model.dispose() }, [model])
  const saved = model.searches.some((item) => item.query.toLocaleLowerCase() === query.trim().toLocaleLowerCase() && item.scope === scope)
  return <VStack gap={8}>
    <HStack wrap gap={8}>
      {model.available ? <ChoicePicker selection={scope} options={searchScopes} onChange={(value) => restore(query, value)} /> : null}
      <Button kind="secondary" disabled={!model.available || model.busy || model.loading || store.isOffline || !query.trim() || saved} onClick={() => void model.save(query, scope)}>{saved ? 'Search saved' : 'Save search'}</Button>
    </HStack>
    {model.loading ? <Text size="caption" tone="secondary">Loading saved searches…</Text> : null}
    {model.error ? <HStack gap={8}><Text size="caption" tone="danger">{model.error}</Text><Button kind="secondary" disabled={model.loading || model.busy || store.isOffline} onClick={() => void model.load()}>Retry saved searches</Button></HStack> : null}
    <div className="saved-history-searches">
      {model.searches.map((item) => <HStack key={item.id} gap={8}>
        <Button kind="secondary" onClick={() => restore(item.query, item.scope)} title={item.query}>{item.query} · {searchScopes.find((candidate) => candidate.id === item.scope)?.label}</Button>
        <IconButton title={`Remove saved search: ${item.query}`} icon="trash" disabled={model.busy || store.isOffline} onClick={() => void model.remove(item.id)} />
      </HStack>)}
    </div>
  </VStack>
}
