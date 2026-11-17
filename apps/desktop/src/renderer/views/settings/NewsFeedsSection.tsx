import { useEffect, useMemo, useState } from 'react'
import { Button, CardSection } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useModel } from '../../lib/observable'
import { useApp } from '../../store/context'
import { Field } from '../marketplace/pieces'

interface Feed {
  id: string
  name: string
  url: string
  items: number
  fetchedAt?: number | null
}

interface Item {
  feed: string
  title: string
  url?: string | null
  published?: number | null
  summary: string
}

const when = (ms?: number | null) =>
  ms ? new Date(ms).toLocaleDateString(undefined, { day: 'numeric', month: 'short' }) : null

/** RSS and Atom feeds this computer follows; the same items a bot reads with `news_feed`. */
export function NewsFeedsSection() {
  const app = useApp()
  const local = useModel(app.local)
  const running = local !== null && local.connection.kind === 'online'
  const [feeds, setFeeds] = useState<Feed[]>([])
  const [items, setItems] = useState<Item[]>([])
  const [name, setName] = useState('')
  const [url, setUrl] = useState('')
  const [only, setOnly] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const load = useMemo(() => async () => {
    if (!local) return
    try {
      const client = await local.ready()
      const [list, news] = await Promise.all([
        client.call<{ feeds: Feed[] }>('feedSubscriptions', {}),
        client.call<{ items: Item[] }>('feedItems', { limit: 60 }),
      ])
      setFeeds(list.feeds)
      setItems(news.items)
      setError(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
  }, [local])

  useEffect(() => {
    if (running) void load()
  }, [running, load])

  const add = async () => {
    if (!local || !url.trim()) return
    setBusy(true)
    try {
      await (await local.ready()).call('addFeed', { name: name.trim() || 'Feed', url: url.trim() }, 45_000)
      setName('')
      setUrl('')
      await load()
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
    setBusy(false)
  }

  const remove = async (feed: Feed) => {
    if (!local) return
    try {
      await (await local.ready()).call('removeFeed', { id: feed.id })
      await load()
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
  }

  const visible = only ? items.filter((item) => item.feed === only) : items

  return (
    <CardSection
      title="News feeds"
      footer="Paste an RSS or Atom address (a blog, a news site, a podcast feed). Codync reads it at most every ten minutes and never writes to the site. Bots read the same items with news_feed."
    >
      {!running ? (
        <Text size="caption" tone="secondary">Start this computer’s host to read its feeds.</Text>
      ) : (
        <VStack gap={10}>
          <VStack gap={6}>
            <HStack gap={8} align="end" wrap>
              <VStack gap={4} grow><Text size="caption" tone="secondary">Name (optional)</Text><Field label="Name" value={name} onChange={setName} placeholder="The Verge" /></VStack>
              <VStack gap={4} grow><Text size="caption" tone="secondary">Feed address</Text><Field label="Feed address" value={url} onChange={setUrl} placeholder="https://example.com/feed.xml" /></VStack>
              <Button kind="secondary" disabled={!url.trim() || busy} onClick={() => void add()}>Add</Button>
            </HStack>
            {feeds.length ? (
              <VStack gap={4}>
                {feeds.map((feed) => (
                  <HStack key={feed.id} gap={8} align="center" className="scheduled-row">
                    <VStack gap={2} grow>
                      <Text weight="semibold">{feed.name}</Text>
                      <Text size="caption" tone="secondary" className="scheduled-text">{feed.url}</Text>
                    </VStack>
                    <Button kind="secondary" disabled={busy} onClick={() => setOnly(only === feed.name ? '' : feed.name)}>
                      {only === feed.name ? 'All feeds' : 'Only this'}
                    </Button>
                    <Button kind="secondary" disabled={busy} onClick={() => void remove(feed)}>Remove</Button>
                  </HStack>
                ))}
              </VStack>
            ) : (
              <Text size="caption" tone="secondary">No news feeds yet.</Text>
            )}
          </VStack>

          <VStack gap={4}>
            <Text size="callout" weight="semibold">{only || 'Newest items'}</Text>
            {visible.length ? visible.map((item) => (
              <VStack key={`${item.feed}-${item.url ?? item.title}-${item.published ?? 0}`} gap={2}>
                <button
                  className="plain-link"
                  style={{ textAlign: 'left' }}
                  onClick={() => { if (item.url) window.codync.app.openExternal(item.url) }}
                >
                  <Text weight="medium">{item.title}</Text>
                </button>
                <Text size="caption" tone="secondary">
                  {item.feed}{when(item.published) ? ` · ${when(item.published)}` : ''}
                </Text>
                {item.summary ? <Text size="caption" tone="tertiary" lines={3}>{item.summary}</Text> : null}
              </VStack>
            )) : <Text size="caption" tone="secondary">No items yet. Add a feed or refresh.</Text>}
          </VStack>
          <HStack gap={8}>
            <Button kind="secondary" disabled={busy} onClick={() => void load()}>Refresh</Button>
          </HStack>
          {error ? <Text size="caption" tone="danger">{error}</Text> : null}
        </VStack>
      )}
    </CardSection>
  )
}
