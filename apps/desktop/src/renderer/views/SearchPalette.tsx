import { useEffect, useRef, useState } from 'react'
import { isGroup, type Bot } from '@shared/models'
import { AvatarWithStatus } from '../components/Avatar'
import { Icon } from '../components/Icon'
import { Sheet } from '../components/Overlay'
import { HStack, VStack } from '../components/Stack'
import { Text } from '../components/Text'
import { refKey, type BotRef, type RosterItem } from '../store/app-model'
import './search-palette.css'
import { matchesRoster, rosterFilters, type RosterFilter } from '../lib/roster-query'
import { RosterHistory } from './RosterControls'
import { ChoicePicker } from '../components/Controls'
import { SearchActions, type SearchAction } from './SearchActions'

const isMac = window.codync.platform === 'darwin'
const modifier = isMac ? '⌘' : 'Ctrl+'

/** Bots matching the query by name, description or last message; all bots for an empty query. */
export function matchBots(items: RosterItem[], query: string, filter: RosterFilter = 'all'): RosterItem[] {
  const q = query.trim().toLowerCase()
  if (!q && filter === 'all') return items
  return items.filter(({ bot }) => matchesRoster(bot, q, filter))
}

/** One result of the global search: a message or a document. */
export interface SearchHit {
  computerId: string
  botId: string
  entryId?: string
  documentId?: string
  kind: string
  text: string
  createdAt: number
}

/** Grok Bot's bot search: a floating list over the window, arrows/Enter or ⌘1-9 to open a bot. */
export function SearchPalette({ open, items, onPick, onClose, actions = [], onSearch }: { open: boolean; items: RosterItem[]; onPick: (ref: BotRef) => void; onClose: () => void; actions?: SearchAction[]; onSearch?: (query: string) => Promise<SearchHit[]> }) {
  return (
    <Sheet open={open} onClose={onClose} width={640} dismissOnDim>
      {open ? <PaletteBody items={items} onPick={onPick} onClose={onClose} actions={actions} onSearch={onSearch} /> : null}
    </Sheet>
  )
}

function PaletteBody({ items, onPick, onClose, actions, onSearch }: { items: RosterItem[]; onPick: (ref: BotRef) => void; onClose: () => void; actions: SearchAction[]; onSearch?: (query: string) => Promise<SearchHit[]> }) {
  const [query, setQuery] = useState('')
  const [mode, setMode] = useState<'bots' | 'messages' | 'history' | 'actions'>('bots')
  const [filter, setFilter] = useState<RosterFilter>('all')
  const [active, setActive] = useState(0)
  const [hits, setHits] = useState<SearchHit[]>([])
  const [searching, setSearching] = useState(false)
  const list = useRef<HTMLDivElement>(null)
  const searchRef = useRef(onSearch)
  searchRef.current = onSearch
  const results = matchBots(items, query, filter)
  const pick = (item: RosterItem | undefined) => {
    if (!item) return
    onPick(item.ref)
    onClose()
  }

  useEffect(() => setActive(0), [query, filter])
  useEffect(() => {
    list.current?.children[active]?.scrollIntoView({ block: 'nearest' })
  }, [active])
  // The global search asks the host after a short pause; two characters are the minimum.
  useEffect(() => {
    const search = searchRef.current
    if (mode !== 'messages' || !search) return
    const q = query.trim()
    if (q.length < 2) {
      setHits([])
      setSearching(false)
      return
    }
    let cancelled = false
    setSearching(true)
    const timer = window.setTimeout(() => {
      void search(q)
        .then((found) => {
          if (!cancelled) setHits(found)
        })
        .catch(() => {
          if (!cancelled) setHits([])
        })
        .finally(() => {
          if (!cancelled) setSearching(false)
        })
    }, 250)
    return () => {
      cancelled = true
      window.clearTimeout(timer)
    }
  }, [query, mode])

  const onKey = (e: React.KeyboardEvent) => {
    if (e.nativeEvent.isComposing) return
    const digit = Number(e.key)
    if ((isMac ? e.metaKey : e.ctrlKey) && digit >= 1 && digit <= 9) {
      e.preventDefault()
      pick(results[digit - 1])
    } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault()
      const step = e.key === 'ArrowDown' ? 1 : -1
      setActive((i) => (results.length ? (i + step + results.length) % results.length : 0))
    } else if (e.key === 'Enter') {
      e.preventDefault()
      pick(results[active])
    }
  }

  return (
    <div className="palette" onKeyDown={mode === 'bots' ? onKey : undefined}>
      <HStack gap={8} justify="space-between">
        <ChoicePicker selection={mode} options={[{ id: 'bots', label: 'Bots' }, ...(onSearch ? [{ id: 'messages' as const, label: 'Everything' }] : []), { id: 'history', label: 'History' }, ...(window.codync.platform === 'win32' ? [{ id: 'actions' as const, label: 'Actions' }] : [])]} onChange={setMode} />
        {mode === 'bots' ? <ChoicePicker selection={filter} options={rosterFilters} onChange={setFilter} /> : null}
      </HStack>
      {mode === 'actions' ? <SearchActions actions={[{ id: 'history', title: 'Search history', icon: 'magnifyingglass', run: () => setMode('history') }, ...actions]} /> : mode === 'history' ? <RosterHistory items={items} onPick={(item) => onPick(item.ref)} onClose={onClose} /> : mode === 'messages' ? <>
        <div className="palette-field">
          <Icon name="magnifyingglass" size={15} color="var(--secondary)" />
          <input autoFocus value={query} placeholder="Search every chat and document" aria-label="Search everything" spellCheck={false} onChange={(e) => setQuery(e.target.value)} />
        </div>
        <div className="palette-list" role="listbox">
          {searching ? <div className="palette-empty">Searching…</div> : null}
          {hits.map((hit) => {
            const item = items.find((it) => it.ref.botId === hit.botId && it.ref.computerId === hit.computerId)
            return (
              <button
                key={`${hit.computerId}-${hit.botId}-${hit.entryId ?? hit.documentId}`}
                role="option"
                aria-selected={false}
                className="palette-row"
                onClick={() => {
                  onPick({ computerId: hit.computerId, botId: hit.botId })
                  onClose()
                }}
              >
                {item ? <AvatarWithStatus bot={item.bot} members={isGroup(item.bot) ? item.store.members(item.bot) : []} size={32} /> : <Icon name={hit.documentId ? 'doc.text' : 'bubble.left'} size={20} />}
                <VStack as="span" gap={2} grow>
                  <Text size="body" tone="text" truncate>{hit.text}</Text>
                  <Text size="subheadline" tone="secondary" truncate>
                    {item?.bot.name ?? hit.botId} · {hit.documentId ? 'Document' : 'Message'}
                  </Text>
                </VStack>
              </button>
            )
          })}
          {!searching && query.trim().length >= 2 && hits.length === 0 ? <div className="palette-empty">No matches</div> : null}
          {query.trim().length < 2 ? <div className="palette-empty">Type at least two characters</div> : null}
        </div>
      </> : <>
      <div className="palette-field">
        <Icon name="magnifyingglass" size={15} color="var(--secondary)" />
        <input autoFocus value={query} placeholder="Search" aria-label="Search bots" spellCheck={false} onChange={(e) => setQuery(e.target.value)} />
      </div>
      <div className="palette-list" ref={list} role="listbox">
        {results.map((item, i) => (
          <button
            key={refKey(item.ref)}
            role="option"
            aria-selected={i === active}
            className={`palette-row ${i === active ? 'active' : ''}`}
            onMouseMove={() => setActive(i)}
            onClick={() => pick(item)}
          >
            <AvatarWithStatus bot={item.bot} members={isGroup(item.bot) ? item.store.members(item.bot) : []} size={32} />
            <VStack as="span" gap={2} grow>
              <Text size="body" tone="text" truncate>{item.bot.name}</Text>
              {subtitle(item) ? <Text size="subheadline" tone="secondary" truncate>{subtitle(item)}</Text> : null}
            </VStack>
            {i < 9 ? <span className="key-cap palette-key">{`${modifier}${i + 1}`}</span> : null}
          </button>
        ))}
        {results.length === 0 ? <div className="palette-empty">No matching bots</div> : null}
      </div>
      </>}
    </div>
  )
}

/** What the bot is for; a group lists its members. */
function subtitle({ bot, store }: { bot: Bot; store: RosterItem['store'] }) {
  if (!isGroup(bot) || bot.description) return bot.description
  const names = store.members(bot).map((m) => m.name).join(', ')
  return names === bot.name ? '' : names
}
