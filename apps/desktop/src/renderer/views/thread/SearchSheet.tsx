import { useEffect, useRef, useState } from 'react'
import type { Entry } from '@shared/models'
import { TabChoice } from '../../components/Controls'
import { Sheet, ModalHeader, SheetPage } from '../../components/Overlay'
import { HStack, Spacer, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { Icon } from '../../components/Icon'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'
import { relativeTime } from './chat-items'
import { MarkdownText } from './Markdown'
import './search-sheet.css'

interface Hit {
  kind: 'message' | 'document' | 'memory'
  id: string
  at: number
  snippet: string
  entryKind?: string | null
  author?: string | null
  title?: string | null
  threadId?: string | null
  shared?: boolean
}
type Results =
  | { kind: 'idle' | 'loading' }
  | { kind: 'error'; message: string }
  | { kind: 'ready'; hits: Hit[] }

type Author = 'all' | 'user' | 'agent'
type Scope = 'all' | 'main' | 'threads'

/**
 * Ask this chat what it already knows: its messages, its pages and its memory, best match first, each with
 * its snippet and date. A message opens in its context; a page or a note opens here, before its library.
 */
export function SearchSheet({ botId, close }: { botId: string; close: () => void }) {
  const store = useStore()
  const tr = useTr()
  const bots = store.bots
  const [query, setQuery] = useState('')
  const [author, setAuthor] = useState<Author>('all')
  const [scope, setScope] = useState<Scope>('all')
  const [tools, setTools] = useState(false)
  const [results, setResults] = useState<Results>({ kind: 'idle' })
  const [page, setPage] = useState<{ title: string; body: string; note: boolean } | null>(null)
  const sequence = useRef(0)
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    const mine = ++sequence.current
    if (timer.current) clearTimeout(timer.current)
    const text = query.trim()
    if (!text) {
      setResults({ kind: 'idle' })
      return
    }
    setResults({ kind: 'loading' })
    timer.current = setTimeout(() => {
      void (async () => {
        try {
          const answer = await (await store.ready()).call<{ hits: Hit[] }>('search', {
            botId, query: text, limit: 30, tools,
            ...(author === 'all' ? {} : { author }),
            ...(scope === 'all' ? {} : { channel: scope }),
          })
          if (mine === sequence.current) setResults({ kind: 'ready', hits: answer.hits })
        } catch (error) {
          if (mine === sequence.current) {
            setResults({ kind: 'error', message: error instanceof Error ? error.message : String(error) })
          }
        }
      })()
    }, 250)
    return () => { if (timer.current) clearTimeout(timer.current) }
  }, [query, author, scope, tools, botId, store])

  const openMessage = async (hit: Hit) => {
    try {
      const answer = await (await store.ready()).call<{ entries: Entry[] }>('messageContext', { botId, entryId: hit.id })
      store.openMessageContext(answer.entries, { botId, id: hit.id, threadId: hit.threadId ?? null })
      close()
    } catch (error) {
      setResults({ kind: 'error', message: error instanceof Error ? error.message : String(error) })
    }
  }
  const openPage = async (hit: Hit) => {
    try {
      const answer = await (await store.ready()).call<{ document: { title: string; body: string } }>('getDocument', { id: hit.id })
      setPage({ ...answer.document, note: false })
    } catch (error) {
      setResults({ kind: 'error', message: error instanceof Error ? error.message : String(error) })
    }
  }
  const openMemory = async (hit: Hit) => {
    try {
      const answer = await (await store.ready()).call<{ content: string }>('memoryFile', {
        botId, file: hit.title ?? '', shared: hit.shared === true,
      })
      setPage({ title: hit.title ?? '', body: answer.content, note: true })
    } catch (error) {
      setResults({ kind: 'error', message: error instanceof Error ? error.message : String(error) })
    }
  }
  const open = (hit: Hit) => void (hit.kind === 'message' ? openMessage(hit) : hit.kind === 'memory' ? openMemory(hit) : openPage(hit))

  return (
    <SheetPage>
      <ModalHeader
        icon="magnifyingglass"
        title={tr('Search this chat')}
        subtitle={tr('What was said, its pages and its memory.')}
        onClose={close}
      />
      <div className="thread-hairline" />
      <VStack gap={10} className="search-sheet-controls">
        <input
          autoFocus
          className="roster-search"
          aria-label={tr('Search this chat')}
          placeholder={tr('Words to look for')}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
        <HStack gap={10} wrap>
          <TabChoice
            label={tr('Where')}
            selection={scope}
            options={[{ id: 'all', label: tr('Everything') }, { id: 'main', label: tr('This chat') }, { id: 'threads', label: tr('Threads') }]}
            onChange={setScope}
          />
          <TabChoice
            label={tr('Who said it')}
            selection={author}
            options={[{ id: 'all', label: tr('Everyone') }, { id: 'user', label: tr('Me') }, { id: 'agent', label: tr('Bot') }]}
            onChange={setAuthor}
          />
          <button className={`search-toggle press ${tools ? 'on' : ''}`} aria-pressed={tools} onClick={() => setTools(!tools)}>
            <Icon name="hammer" size={11} />
            {tr('Tool traces')}
          </button>
        </HStack>
      </VStack>
      <VStack gap={10} className="roster-history">
        {results.kind === 'idle' ? <Text tone="secondary" size="caption">{tr('Type a few words. Every one of them has to appear.')}</Text> : null}
        {results.kind === 'loading' ? <Text tone="secondary" size="caption">{tr('Searching…')}</Text> : null}
        {results.kind === 'error' ? <Text role="alert" tone="danger">{results.message}</Text> : null}
        {results.kind === 'ready' && !results.hits.length ? (
          <Text tone="secondary" size="caption">{tr('Nothing here matches those words.')}</Text>
        ) : null}
        {results.kind === 'ready' ? results.hits.map((hit) => (
          <button key={`${hit.kind}:${hit.id}`} className="roster-hit" onClick={() => open(hit)}>
            <HStack gap={8} className="search-hit-head">
              <Icon
                name={
                  hit.kind === 'document' ? 'doc.text'
                    : hit.kind === 'memory' ? 'brain'
                      : hit.entryKind === 'user' ? 'person' : 'bubble.left'
                }
                size={11}
              />
              <Text size="caption" tone="secondary" truncate>
                {hit.kind === 'document'
                  ? `${hit.title ?? tr('Page')} · ${tr('page')}`
                  : hit.kind === 'memory'
                    ? `${hit.title ?? tr('Note')} · ${hit.shared ? tr('shared memory') : tr('memory')}`
                    : `${hit.entryKind === 'user' ? tr('You') : bots.get(botId)?.name ?? tr('Bot')}${hit.author ? ` (${hit.author})` : ''}`}
              </Text>
              <Spacer />
              <Text size="caption" tone="tertiary">{relativeTime.day(hit.at, tr)}</Text>
            </HStack>
            <Text as="div" className="search-snippet">{snippet(hit.snippet)}</Text>
          </button>
        )) : null}
      </VStack>
      <Sheet open={page !== null} onClose={() => setPage(null)} width={560} height={480} dismissOnDim>
        <ModalHeader
          title={page?.title ?? ''}
          subtitle={page?.note ? tr('A note from its memory') : tr('A page of this bot')}
          icon={page?.note ? 'brain' : 'doc.text'}
          onClose={() => setPage(null)}
        />
        <div className="roster-history"><MarkdownText text={page?.body ?? ''} /></div>
      </Sheet>
    </SheetPage>
  )
}

/** The snippet with its matches marked: `[word]` comes back bold, the elisions stay plain. */
function snippet(text: string) {
  return text.split(/(\[[^\]]*\])/).map((part, index) =>
    part.startsWith('[') && part.endsWith(']')
      ? <mark key={index} className="search-mark">{part.slice(1, -1)}</mark>
      : <span key={index}>{part}</span>)
}
