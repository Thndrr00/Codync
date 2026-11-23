import { useState } from 'react'
import { Button, CardSection, IconButton } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { waitingCounts, type WaitState } from '../../lib/waiting-counts.ts'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'
import { waitingReplyRows } from './waiting-replies'
import './waiting-replies.css'

const STATE_LABEL: Record<WaitState, string> = {
  asked: 'Asked',
  stopped: 'Stopped',
  failed: 'Failed',
  replied: 'Replied',
}

/**
 * The list above a chat's (or thread's) composer: one row per teammate the agent asked, with a Stop
 * on each row. Stopping drops the request from the teammate's queue on the host and leaves a
 * "Stopped" row. When every request ended and one failed, the header says "Requests ended" with the
 * replied and failed counts, and the failed rows stay until the list is closed (OpenBot nightly
 * triage-13388).
 */
export function WaitingRepliesCard({ botId, threadId = null }: { botId: string; threadId?: string | null }) {
  const store = useStore()
  const tr = useTr()
  const [hidden, setHidden] = useState(false)
  const stopped = new Set(store.stoppedRequests.keys())
  const entries = [...store.allEntries(botId), ...store.stoppedRequests.values()]
  const all = waitingReplyRows(entries, botId, threadId, stopped, (id) => store.authorName(id))
  const counts = waitingCounts(all)
  // Replied rows have served their purpose; failed ones stay visible in the ended list.
  const rows = all.filter((row) => row.state !== 'replied')
  if (hidden || !rows.length) return null
  const allStopped = rows.every((row) => row.state === 'stopped')
  const dismissable = allStopped || counts.ended
  const title = counts.ended ? tr('Requests ended') : tr('Waiting for replies')
  const count = counts.ended
    ? tr('{done} replied · {failed} failed').replaceAll('{done}', String(counts.replied)).replaceAll('{failed}', String(counts.failed))
    : `${counts.asked}/${counts.total}`
  return (
    <div className="waiting-replies">
      <CardSection
        title={
          <HStack as="span" gap={6} align="center">
            <Text as="span" weight="semibold">{title}</Text>
            <Text as="span" size="caption" tone={counts.ended ? 'danger' : 'secondary'}>{count}</Text>
          </HStack>
        }
        accessory={dismissable ? <Button kind="secondary" onClick={() => setHidden(true)}>{tr('Hide this list')}</Button> : undefined}>
        <VStack gap={6}>
          {rows.map((row) => (
            <HStack key={row.id} gap={8} align="center" className="waiting-reply-row">
              <VStack gap={1} shrink>
                <Text size="subheadline" weight="semibold">{row.name}</Text>
                <Text size="caption" tone="secondary" lines={1}>{row.request}</Text>
              </VStack>
              <Text size="caption" tone={row.state === 'failed' ? 'danger' : row.state === 'stopped' ? 'tertiary' : 'accent'}>{tr(STATE_LABEL[row.state])}</Text>
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
