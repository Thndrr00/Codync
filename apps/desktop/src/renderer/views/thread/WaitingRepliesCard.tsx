import { useState } from 'react'
import { Button, CardSection, IconButton } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'
import { waitingReplyRows } from './waiting-replies'
import './waiting-replies.css'

/**
 * The "Waiting for replies" card above a chat's (or thread's) composer: one row per teammate the
 * agent asked, with a Stop on each row. Stopping drops the request from the teammate's queue on
 * the host and leaves a "Stopped" row. When nothing waits, "Hide this list" closes the card.
 */
export function WaitingRepliesCard({ botId, threadId = null }: { botId: string; threadId?: string | null }) {
  const store = useStore()
  const tr = useTr()
  const [hidden, setHidden] = useState(false)
  const stopped = new Set(store.stoppedRequests.keys())
  const entries = [...store.allEntries(botId), ...store.stoppedRequests.values()]
  const rows = waitingReplyRows(entries, botId, threadId, stopped, (id) => store.authorName(id))
  if (hidden || !rows.length) return null
  const allStopped = rows.every((row) => row.state === 'stopped')
  return (
    <div className="waiting-replies">
      <CardSection title={tr('Waiting for replies')}
        accessory={allStopped ? <Button kind="secondary" onClick={() => setHidden(true)}>{tr('Hide this list')}</Button> : undefined}>
        <VStack gap={6}>
          {rows.map((row) => (
            <HStack key={row.id} gap={8} align="center" className="waiting-reply-row">
              <VStack gap={1} shrink>
                <Text size="subheadline" weight="semibold">{row.name}</Text>
                <Text size="caption" tone="secondary" lines={1}>{row.request}</Text>
              </VStack>
              <Text size="caption" tone={row.state === 'stopped' ? 'tertiary' : 'accent'}>{tr(row.state === 'stopped' ? 'Stopped' : 'Asked')}</Text>
              {row.state === 'asked' ? <IconButton title={tr('Stop')} icon="xmark" onClick={() => {
                void store.stopRequest(row.entry)
              }} /> : null}
            </HStack>
          ))}
        </VStack>
      </CardSection>
    </div>
  )
}
