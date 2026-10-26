import { useState } from 'react'
import type { Bot } from '@shared/models'
import {
  MAX_TEAM_RUN_STAGES, teamRunIssue, teamRunMessage, type TeamRunDraft, type TeamRunMode, type TeamRunStage,
} from '@shared/team-run'
import { Button, CardSection, ChoicePicker, IconButton, ValueRow } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { Field } from '../marketplace/pieces'

const MODES: { id: TeamRunMode; label: string }[] = [
  { id: 'pipeline', label: 'One after another' },
  { id: 'parallel', label: 'At the same time' },
]

/** A job for several bots, written here and sent to one bot that runs it with its pipeline or batch tool. */
export function TeamRunSection({ bots, online, send }: { bots: Bot[]; online: boolean; send: (text: string, botId: string) => void }) {
  const [coordinator, setCoordinator] = useState('')
  const [draft, setDraft] = useState<TeamRunDraft>({ mode: 'pipeline', input: '', stages: [{ botId: '', goal: '' }] })
  const [sentTo, setSentTo] = useState<string | null>(null)
  const botName = (id: string) => bots.find((b) => b.id === id)?.name ?? 'Unknown bot'
  const lead = bots.some((b) => b.id === coordinator) ? coordinator : (bots[0]?.id ?? '')
  const issue = teamRunIssue(draft, lead)
  const botOptions = [{ id: '', label: 'Choose a bot' }, ...bots.filter((b) => b.id !== lead).map((b) => ({ id: b.id, label: b.name }))]
  const setStage = (index: number, patch: Partial<TeamRunStage>) => {
    setSentTo(null)
    setDraft((d) => ({ ...d, stages: d.stages.map((s, i) => (i === index ? { ...s, ...patch } : s)) }))
  }
  const dropStage = (index: number) => setDraft((d) => ({ ...d, stages: d.stages.filter((_, i) => i !== index) }))
  const submit = () => {
    send(teamRunMessage(draft, botName), lead)
    setSentTo(botName(lead))
    setDraft({ mode: draft.mode, input: '', stages: [{ botId: '', goal: '' }] })
  }
  return (
    <CardSection title="Team run"
      footer="Write the job here; the coordinator bot runs it with its pipeline or batch tool and answers in its chat. It uses the bots' own folders and permissions.">
      <ValueRow label="Coordinator">
        <ChoicePicker selection={lead} options={bots.map((b) => ({ id: b.id, label: b.name }))} onChange={setCoordinator} />
      </ValueRow>
      <ValueRow label="Stages run">
        <ChoicePicker selection={draft.mode} options={MODES} onChange={(mode) => setDraft((d) => ({ ...d, mode }))} />
      </ValueRow>
      {draft.mode === 'pipeline' ? (
        <Field label="Input for the first stage" value={draft.input} onChange={(input) => setDraft((d) => ({ ...d, input }))} placeholder="Optional: what stage 1 gets as {{previous}}" />
      ) : null}
      {draft.stages.map((stage, index) => (
        <VStack key={index} gap={6} className="settings-row">
          <HStack gap={8}>
            <Text weight="semibold">{draft.mode === 'pipeline' ? `Stage ${index + 1}` : `Task ${index + 1}`}</Text>
            <ChoicePicker selection={stage.botId} options={botOptions} onChange={(botId) => setStage(index, { botId })} />
            {draft.stages.length > 1 ? <IconButton title={`Remove stage ${index + 1}`} icon="xmark" onClick={() => dropStage(index)} /> : null}
          </HStack>
          <Field label={`Goal of stage ${index + 1}`} value={stage.goal} onChange={(goal) => setStage(index, { goal })} lines={[2, 6]}
            placeholder={draft.mode === 'pipeline' && index > 0 ? 'What to do. Use {{previous}} for the last reply.' : 'What this bot should do'} />
        </VStack>
      ))}
      <HStack gap={8}>
        <Button kind="secondary" disabled={draft.stages.length >= MAX_TEAM_RUN_STAGES}
          onClick={() => setDraft((d) => ({ ...d, stages: [...d.stages, { botId: '', goal: '' }] }))}>
          {draft.mode === 'pipeline' ? 'Add stage' : 'Add task'}
        </Button>
        <Button disabled={!online || issue !== null} onClick={submit}>Send to {botName(lead)}</Button>
      </HStack>
      {issue && draft.stages.some((s) => s.botId || s.goal) ? <Text size="footnote" tone="secondary">{issue}</Text> : null}
      {sentTo ? <Text as="div" role="status" size="footnote" tone="secondary">Sent to {sentTo}. Follow it in its chat.</Text> : null}
    </CardSection>
  )
}
