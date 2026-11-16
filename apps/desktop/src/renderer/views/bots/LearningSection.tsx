import { useCallback, useEffect, useState } from 'react'
import { Button, CardSection, ChipChoice } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { learnedTitle, hitsLabel, type LearningInbox } from '../../../shared/learning'
import { useStore } from '../../store/context'
import { SwitchRow } from './parts'

const REVIEW_CHOICES = [0, 2, 3, 5]

/** What a bot learned from the person's corrections and repeated instructions: nothing is memory until approved. */
export function LearningSection({ botId }: { botId: string }) {
  const host = useStore()
  const tr = useTr()
  const [inbox, setInbox] = useState<LearningInbox | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const call = useCallback(async (method: string, extra: Record<string, unknown> = {}) => {
    setBusy(true)
    try {
      const reply = await (await host.ready()).call<LearningInbox>(method, { botId, ...extra })
      setInbox(reply)
      setError(null)
    } catch (e) { setError(e instanceof Error ? e.message : String(e)) }
    finally { setBusy(false) }
  }, [host, botId])
  useEffect(() => { void call('learning') }, [call])
  if (inbox === null && error === null) return null
  const list = inbox?.proposals ?? []
  return <CardSection title={learnedTitle(list.length)} footer={`Corrections and instructions you give this bot can become one line of its ${inbox?.note ?? 'preferences.md'} note. Nothing is saved until you approve it, unless you let it save after repeats. The bot's replies, routines and group chats are never read for this.`}>
    {error ? <Text size="footnote" tone="danger">{error}</Text> : null}
    {inbox ? <>
      <SwitchRow title="Learn from my corrections and instructions" detail="Proposes durable preferences, never one-off tasks or secrets." on={inbox.enabled} disabled={busy}
        onChange={(enabled) => void call('saveLearningSettings', { enabled })} />
      <ChipChoice label="Save without review after" selection={inbox.autoApproveAfter}
        options={REVIEW_CHOICES.map((n) => ({ id: n, label: n === 0 ? 'Off' : `${n}×` }))}
        onChange={(autoApproveAfter) => void call('saveLearningSettings', { autoApproveAfter })} />
    </> : null}
    {list.length === 0 && !error ? <Text tone="secondary">{tr('Nothing to review.')}</Text> : null}
    {list.map((p) => <VStack key={p.id} gap={4}>
      <Text>{p.text}</Text>
      <Text size="caption" tone="tertiary">{hitsLabel(p)}</Text>
      <Text size="footnote" tone="secondary" truncate>{tr('From')}: “{p.source}”</Text>
      <HStack gap={8}>
        <Button disabled={busy} onClick={() => void call('approveLearning', { id: p.id })}>{tr('Remember this')}</Button>
        <Button kind="secondary" disabled={busy} onClick={() => void call('dismissLearning', { id: p.id })}>{tr('Dismiss')}</Button>
      </HStack>
    </VStack>)}
  </CardSection>
}
