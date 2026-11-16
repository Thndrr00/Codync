import type { ConversationLink } from '../../../shared/channels'
import { useEffect, useState } from 'react'
import { Button, ChoicePicker, IconButton } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import type { ChannelsStore } from '../../store/channels-store'

/** The channels whose conversations can share a Codync thread (card K13). */
type LinkableProvider = 'telegram' | 'slack' | 'discord'

/**
 * The Codync thread a channel conversation shares: what is linked now, a way to link one, and to take it away.
 * The host keys a conversation by its own ids (a Telegram chat id, a Slack thread timestamp, a Discord channel),
 * so only Discord's is the channel this page shows; the other two are typed in.
 */
export function ChannelLinks({ provider, channel, botId, store }: {
  provider: LinkableProvider
  /** The channel this page shows: a Discord channel's id. */
  channel: string
  botId: string
  store: ChannelsStore
}) {
  const tr = useTr()
  const [links, setLinks] = useState<ConversationLink[]>([])
  const [threads, setThreads] = useState<{ id: string; name: string }[]>([])
  const [conversation, setConversation] = useState('')
  const [choice, setChoice] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  useEffect(() => {
    let live = true
    void Promise.all([store.conversationLinks(botId), store.threads(botId)]).then(([found, list]) => {
      if (!live) return
      setLinks(found)
      setThreads(list)
    }, () => {})
    return () => {
      live = false
    }
  }, [botId, store])
  const run = async (work: () => Promise<ConversationLink[]>) => {
    setBusy(true)
    try {
      setLinks(await work())
      setError(null)
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason))
    } finally {
      setBusy(false)
    }
  }
  // A Telegram chat or a Slack thread is named by the host's own id; a Discord channel is the one on this page.
  const key = provider === 'discord'
    ? { channel, thread: '' }
    : provider === 'slack'
      ? { channel, thread: conversation.trim() }
      : { channel: conversation.trim(), thread: '' }
  const name = (rootId: string) => threads.find((thread) => thread.id === rootId)?.name ?? rootId
  // A page shows its own channel's links; a Telegram bot's conversations are all its chats, which it names by id.
  const mine = links.filter((link) => link.source === provider && (provider === 'telegram' || link.channel === channel))
  const link = () => void run(() => store.linkConversation({ botId, provider, channel: key.channel, thread: key.thread, threadId: choice }))
  return (
    <VStack gap={4}>
      <Text size="caption" tone="tertiary">{tr('Linked conversation')}</Text>
      {mine.map((item) => (
        <HStack key={`${item.channel}/${item.thread}`} gap={8}>
          <VStack gap={2} grow>
            <Text size="caption">{tr('Linked to {thread}').replace('{thread}', name(item.rootId))}</Text>
            <Text size="caption" tone="tertiary">{item.thread ? `${item.channel} · ${item.thread}` : item.channel}</Text>
          </VStack>
          <IconButton title={tr('Unlink')} icon="minus.circle" size={14} disabled={busy}
            onClick={() => void run(() => store.unlinkConversation({ botId, provider, channel: item.channel, thread: item.thread }))} />
        </HStack>
      ))}
      {provider === 'discord' ? null : (
        <input className="field-box" value={conversation} spellCheck={false} aria-label={tr('Channel conversation')}
          placeholder={tr('Its id in the channel: a Telegram chat id, a Slack thread timestamp')}
          onChange={(event) => setConversation(event.target.value)} />
      )}
      <HStack gap={6}>
        <ChoicePicker selection={choice} options={threads.map((thread) => ({ id: thread.id, label: thread.name }))}
          onChange={setChoice} />
        <Button kind="secondary" disabled={busy || !choice || !key.channel} onClick={link}>{tr('Link a thread')}</Button>
      </HStack>
      {error ? <Text size="caption" role="alert" tone="danger">{error}</Text> : null}
    </VStack>
  )
}
