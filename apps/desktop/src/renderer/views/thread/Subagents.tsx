import { useEffect, useState } from 'react'
import type { Entry, Subagent } from '@shared/models'
import { Icon } from '../../components/Icon'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { subagentDuration, subagentState } from './subagent-card'

/** The time now, once a second while `running`. */
function useNow(running: boolean): number {
  const [now, setNow] = useState(() => Date.now())
  useEffect(() => {
    setNow(Date.now())
    if (!running) return
    const timer = window.setInterval(() => setNow(Date.now()), 1_000)
    return () => window.clearInterval(timer)
  }, [running])
  return now
}

function elapsed(subagent: Subagent, now: number): number {
  return (subagent.endedAt ?? now) - subagent.startedAt
}

function StateIcon({ subagent }: { subagent: Subagent }) {
  const state = subagentState(subagent)
  const color =
    state === 'failed' ? 'var(--danger)' : state === 'running' ? 'var(--accent)' : 'var(--secondary)'
  const name = state === 'running' ? 'circle.dotted' : state === 'done' ? 'checkmark.circle.fill' : state === 'failed' ? 'xmark.circle.fill' : 'minus.circle'
  return <Icon name={name} size={13} color={color} />
}

/** The last thing a subagent did, or how it ended. */
function stepText(subagent: Subagent, tr: (text: string) => string): string {
  const state = subagentState(subagent)
  if (state === 'failed') return subagent.step || tr('Failed')
  if (state === 'stopped') return subagent.step || tr('Stopped')
  if (state === 'done') return subagent.step || tr('Done')
  return subagent.step || tr('Working…')
}

/** A ring of the subagents that finished, and "{finished} of {total}". */
function Progress({ finished, total }: { finished: number; total: number }) {
  const tr = useTr()
  const radius = 7
  const circumference = 2 * Math.PI * radius
  const filled = (finished / Math.max(1, total)) * circumference
  return (
    <HStack as="span" gap={4} align="center" className="subagent-progress">
      <svg className="subagent-ring" viewBox="0 0 18 18" aria-hidden="true">
        <circle className="subagent-ring-track" cx="9" cy="9" r={radius} />
        <circle className="subagent-ring-fill" cx="9" cy="9" r={radius} strokeDasharray={`${filled} ${circumference}`} />
      </svg>
      <Text size="caption2" tone="secondary" tabular>{tr('{finished} of {total}').replaceAll('{finished}', String(finished)).replaceAll('{total}', String(total))}</Text>
    </HStack>
  )
}

/**
 * The subagents one turn started: a header with their progress and time, and a row for each with
 * its task, what it does now and how long it has taken. The list opens while one works.
 */
export function SubagentCard({ entry }: { entry: Entry }) {
  const tr = useTr()
  const subagents = entry.data.subagents ?? []
  const running = subagents.filter((s) => subagentState(s) === 'running').length
  const failed = subagents.filter((s) => subagentState(s) === 'failed').length
  const finished = subagents.length - running
  const now = useNow(running > 0)
  const start = Math.min(...subagents.map((s) => s.startedAt))
  const end = running > 0 ? now : Math.max(...subagents.map((s) => s.endedAt ?? now))
  const [choice, setChoice] = useState<boolean>()
  const open = choice ?? running > 0
  if (!subagents.length) return null
  return (
    <VStack gap={4} className="subagent-card">
      <button
        className="press subagent-card-header"
        aria-expanded={open}
        onClick={() => setChoice(!open)}
      >
        <Text size="subheadline" weight="medium">{tr('Subagents')}</Text>
        <Progress finished={finished} total={subagents.length} />
        {failed > 0 ? <Text size="caption2" tone="danger">{tr('Failed')}: {failed}</Text> : null}
        <Text size="caption2" tone="tertiary" tabular>{subagentDuration(end - start)}</Text>
        <Icon name={open ? 'chevron.down' : 'chevron.right'} size={10} color="var(--tertiary)" />
      </button>
      {open ? (
        <VStack gap={2} className="subagent-list">
          {subagents.map((subagent) => (
            <HStack key={subagent.id} gap={8} align="flex-start" className="subagent-row">
              <span className="subagent-row-icon"><StateIcon subagent={subagent} /></span>
              <VStack gap={1} className="subagent-row-copy">
                <Text size="footnote" tone="text" truncate>{subagent.task || tr('Subagents')}</Text>
                <Text size="caption2" tone="secondary" truncate>{stepText(subagent, tr)}</Text>
              </VStack>
              <Text size="caption2" tone="tertiary" tabular>{subagentDuration(elapsed(subagent, now))}</Text>
            </HStack>
          ))}
        </VStack>
      ) : null}
    </VStack>
  )
}

/**
 * The header of a subagent card the reader scrolled away from while its subagents work: what they
 * do now, how far they are and how long they have taken. It takes no height over the chat, and a
 * click goes back to the card.
 */
export function SubagentDock({ card, onReveal }: { card: Entry; onReveal: () => void }) {
  const tr = useTr()
  const subagents = card.data.subagents ?? []
  const running = subagents.filter((s) => subagentState(s) === 'running')
  const now = useNow(running.length > 0)
  const start = Math.min(...subagents.map((s) => s.startedAt))
  const end = running.length > 0 ? now : Math.max(...subagents.map((s) => s.endedAt ?? now))
  const current = running[0]
  return (
    <div className="subagent-dock">
      <button
        className="press subagent-dock-bar"
        aria-label={tr('Go to the subagents')}
        onClick={onReveal}
      >
        <Text size="footnote" weight="medium">{tr('Subagents')}</Text>
        <Progress finished={subagents.length - running.length} total={subagents.length} />
        {current ? (
          <HStack as="span" gap={4} align="center" className="subagent-dock-live">
            <Icon name="circle.dotted" size={10} color="var(--accent)" />
            <Text size="caption2" tone="secondary" truncate>{stepText(current, tr)}</Text>
          </HStack>
        ) : null}
        <Text size="caption2" tone="tertiary" tabular>{subagentDuration(end - start)}</Text>
        <Icon name="chevron.right" size={10} color="var(--tertiary)" />
      </button>
    </div>
  )
}
