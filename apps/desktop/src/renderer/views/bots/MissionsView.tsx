import { useCallback, useEffect, useMemo, useState } from 'react'
import type { Mission, MissionStatus } from '@shared/models'
import { Button, CardSection, Disclosure, DropdownMenu, IconButton, Spinner } from '../../components/Controls'
import { Dialog } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { ToastContainer, useToasts } from '../../components/ToastContainer'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'
import './missions.css'

const LABELS: Record<MissionStatus, string> = {
  proposed: 'Proposed',
  active: 'Running',
  waiting: 'Waiting',
  done: 'Done',
  abandoned: 'Abandoned',
}

const TONES: Record<MissionStatus, string> = {
  proposed: 'missions-pill proposed',
  active: 'missions-pill active',
  waiting: 'missions-pill waiting',
  done: 'missions-pill done',
  abandoned: 'missions-pill abandoned',
}

/** The status a mission moves to when somebody pushes it on, and the verb for that move. */
const NEXT: Record<MissionStatus, { status: MissionStatus; label: string }> = {
  proposed: { status: 'active', label: 'Activate' },
  active: { status: 'waiting', label: 'Pause' },
  waiting: { status: 'active', label: 'Resume' },
  done: { status: 'active', label: 'Reopen' },
  abandoned: { status: 'active', label: 'Reopen' },
}

const returned = (status: MissionStatus) => status === 'done' || status === 'abandoned'

/** One bot's missions: the objective that outlives a turn, its steps, and how far it got. */
export function MissionsCard({ botId }: { botId: string }) {
  const tr = useTr()
  const store = useStore()
  const toasts = useToasts()
  const [missions, setMissions] = useState<Mission[]>([])
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [showFinished, setShowFinished] = useState(false)
  const [composing, setComposing] = useState(false)
  const [title, setTitle] = useState('')
  const [goal, setGoal] = useState('')
  const [steps, setSteps] = useState('')
  const [abandoning, setAbandoning] = useState<Mission | null>(null)
  const [reason, setReason] = useState('')

  const refresh = useCallback(async () => {
    setLoading(true)
    try {
      const open = await store.missions(botId, true).catch(() => [])
      const all = showFinished ? await store.missions(botId).catch(() => []) : []
      const merged = new Map<string, Mission>()
      for (const mission of [...open, ...all]) merged.set(mission.id, mission)
      setMissions([...merged.values()].sort((a, b) => b.updatedAt - a.updatedAt))
    } finally {
      setLoading(false)
    }
  }, [store, botId, showFinished])

  useEffect(() => {
    void refresh()
  }, [refresh])

  const open = useMemo(() => missions.filter((mission) => !returned(mission.status)), [missions])
  const finished = useMemo(() => missions.filter((mission) => returned(mission.status)), [missions])

  /** Runs one change and reloads; the message a refusal carries is the toast, not a silent no-op. */
  const run = useCallback(async (work: () => Promise<unknown>) => {
    setBusy(true)
    try {
      await work()
      await refresh()
    } catch (error) {
      toasts.push(tr(error instanceof Error ? error.message : 'That did not work'), 'danger')
    } finally {
      setBusy(false)
    }
  }, [refresh, toasts, tr])

  const create = () => run(async () => {
    if (!title.trim()) return
    await store.createMission(botId, title.trim(), goal.trim(), steps.split('\n').map((s) => s.trim()).filter(Boolean))
    setTitle('')
    setGoal('')
    setSteps('')
    setComposing(false)
  })

  const abandon = () => run(async () => {
    if (!abandoning) return
    await store.updateMission(abandoning.id, { status: 'abandoned', stopReason: reason.trim() })
    setAbandoning(null)
    setReason('')
  })

  const askAbandon = (mission: Mission) => {
    setAbandoning(mission)
    setReason('')
  }

  return (
    <div className="bot-form">
      <ToastContainer toasts={toasts.toasts} onDismiss={toasts.dismiss} />
      <div className="bot-form-content">
        <CardSection
          title={tr('Missions')}
          accessory={<IconButton title={tr('New mission')} icon="plus" onClick={() => setComposing((v) => !v)} />}
          footer={tr('A mission is a goal that outlives one turn: the bot records what is done and what is left, so the next turn resumes instead of guessing.')}
        >
          <Disclosure expanded={composing} onToggle={() => setComposing((v) => !v)} label={tr('New mission')}>
            <VStack gap={8}>
              <input className="missions-input" aria-label={tr('Mission title')} placeholder={tr('What is the objective?')} value={title} onChange={(event) => setTitle(event.target.value)} />
              <textarea className="missions-input" aria-label={tr('Mission goal')} placeholder={tr('Why, and what counts as done')} rows={2} value={goal} onChange={(event) => setGoal(event.target.value)} />
              <textarea className="missions-input" aria-label={tr('Mission steps')} placeholder={tr('One step per line, if you know them')} rows={3} value={steps} onChange={(event) => setSteps(event.target.value)} />
              <Button disabled={busy || !title.trim()} onClick={() => void create()}>{tr('Create mission')}</Button>
            </VStack>
          </Disclosure>
          {loading && missions.length === 0 ? (
            <HStack justify="center" style={{ padding: 12 }}><Spinner size={16} /></HStack>
          ) : open.length === 0 ? (
            <Text tone="secondary">{tr('No open missions.')}</Text>
          ) : (
            open.map((mission) => (
              <MissionRow key={mission.id} mission={mission} busy={busy} work={run} onAbandon={askAbandon} />
            ))
          )}
        </CardSection>

        {finished.length > 0 ? (
          <CardSection
            title={tr('Finished')}
            accessory={<IconButton title={tr('Show finished')} icon={showFinished ? 'chevron.up' : 'chevron.down'} onClick={() => setShowFinished((v) => !v)} />}
          >
            {showFinished ? finished.map((mission) => (
              <VStack key={mission.id} gap={4}>
                <HStack align="center" gap={6}>
                  <span className={TONES[mission.status]}>{tr(LABELS[mission.status])}</span>
                  <Text weight="medium" truncate grow>{mission.title}</Text>
                </HStack>
                {mission.stopReason ? <Text size="caption" tone="secondary">{mission.stopReason}</Text> : null}
                <HStack gap={8}>
                  <Button kind="secondary" disabled={busy} onClick={() => void run(() => store.updateMission(mission.id, { status: 'active' }))}>{tr('Reopen')}</Button>
                  <Button kind="secondary" disabled={busy} onClick={() => askAbandon(mission)}>{tr('Abandon')}</Button>
                </HStack>
              </VStack>
            )) : <Text tone="secondary">{tr('Finished missions are hidden.')}</Text>}
          </CardSection>
        ) : null}

        <Dialog
          open={!!abandoning}
          title={tr('Abandon mission')}
          message={abandoning?.title ?? null}
          actions={[{ title: tr('Abandon'), destructive: true, action: () => void abandon() }]}
          onClose={() => setAbandoning(null)}
        >
          <input className="missions-input" data-autofocus aria-label={tr('Why it stopped')} placeholder={tr('Why it stopped')} value={reason} onChange={(event) => setReason(event.target.value)} />
        </Dialog>
      </div>
    </div>
  )
}

function MissionRow({ mission, busy, work, onAbandon }: {
  mission: Mission
  busy: boolean
  work: (work: () => Promise<unknown>) => Promise<void>
  onAbandon: (mission: Mission) => void
}) {
  const tr = useTr()
  const store = useStore()
  const [checkpoint, setCheckpoint] = useState(mission.checkpoint)
  const [draftStep, setDraftStep] = useState('')
  useEffect(() => setCheckpoint(mission.checkpoint), [mission.checkpoint])
  const next = NEXT[mission.status]
  return (
    <div className="missions-row">
      <HStack align="center" gap={8}>
        <span className={TONES[mission.status]}>{tr(LABELS[mission.status])}</span>
        <Text weight="semibold" truncate grow>{mission.title}</Text>
        <DropdownMenu title={tr('Mission actions')} className="styled-button secondary" items={() => [
          { title: tr(next.label), action: () => void work(() => store.updateMission(mission.id, { status: next.status })) },
          ...(returned(mission.status) ? [] : [
            { title: tr('Finish'), action: () => void work(() => store.updateMission(mission.id, { status: 'done' })) },
            { title: tr('Abandon'), action: () => onAbandon(mission) },
          ]),
        ]}>{tr('Actions')}</DropdownMenu>
      </HStack>
      {mission.goal && mission.goal !== mission.title ? <Text size="caption" tone="secondary">{mission.goal}</Text> : null}
      {mission.stopReason ? <Text size="caption" tone="secondary">{mission.stopReason}</Text> : null}

      <VStack gap={4}>
        {mission.steps.map((step) => (
          <HStack key={step.id} align="center" gap={8}>
            <Text size="caption" tone="secondary" className="missions-mark">{step.status === 'done' ? '✓' : step.status === 'skipped' ? '–' : step.status === 'active' ? '▸' : '·'}</Text>
            <Text size="caption" grow className={step.status === 'done' ? 'missions-step done' : 'missions-step'}>{step.text}</Text>
            {step.status === 'done' ? null : (
              <>
                <IconButton title={tr('Start step')} icon="play" disabled={busy || step.status === 'active'} onClick={() => void work(() => store.setMissionStep(mission.id, step.id, 'active'))} />
                <IconButton title={tr('Mark done')} icon="checkmark.circle" disabled={busy} onClick={() => void work(() => store.setMissionStep(mission.id, step.id, 'done'))} />
                <IconButton title={tr('Skip step')} icon="forward" disabled={busy} onClick={() => void work(() => store.setMissionStep(mission.id, step.id, 'skipped'))} />
              </>
            )}
          </HStack>
        ))}
      </VStack>

      <HStack gap={8} align="end">
        <input
          className="missions-input"
          aria-label={tr('New step')}
          placeholder={tr('Add a step')}
          value={draftStep}
          onChange={(event) => setDraftStep(event.target.value)}
          onKeyDown={(event) => {
            if (event.key !== 'Enter' || !draftStep.trim()) return
            void work(() => store.addMissionStep(mission.id, draftStep.trim())).then(() => setDraftStep(''))
          }}
        />
        <IconButton title={tr('Add step')} icon="plus" disabled={busy || !draftStep.trim()} onClick={() => void work(() => store.addMissionStep(mission.id, draftStep.trim())).then(() => setDraftStep(''))} />
      </HStack>

      <HStack gap={8} align="end">
        <textarea className="missions-input" aria-label={tr('Checkpoint')} placeholder={tr('What is done and what is left')} rows={2} value={checkpoint} onChange={(event) => setCheckpoint(event.target.value)} />
        <IconButton title={tr('Save checkpoint')} icon="square.and.arrow.down" disabled={busy || checkpoint === mission.checkpoint} onClick={() => void work(() => store.updateMission(mission.id, { checkpoint }))} />
      </HStack>
    </div>
  )
}
