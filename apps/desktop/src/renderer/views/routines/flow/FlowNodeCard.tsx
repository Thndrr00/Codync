import { useEffect, useState, type CSSProperties, type KeyboardEvent, type PointerEvent } from 'react'
import type { Bot } from '@shared/models'
import { BotAvatar } from '../../../components/Avatar'
import { Icon } from '../../../components/Icon'
import { HStack, VStack } from '../../../components/Stack'
import { Text } from '../../../components/Text'
import { useTr } from '../../../lib/use-language'
import { routineTint, statusLabel, mediaTitle, type FlowMedia, type FlowNode, type FlowRoutineCard, type FlowStep, type StepStatus } from '../../../lib/routine-flow.ts'

const STATUS_ICON: Record<StepStatus, string> = {
  queued: 'clock',
  running: 'arrow.triangle.2.circlepath',
  succeeded: 'checkmark.circle',
  failed: 'xmark.circle',
  skipped: 'minus.circle',
  cancelled: 'stop.circle',
}

const DAY_MS = 24 * 60 * 60 * 1000

/** A step's status as a small chip: icon and words, never color alone. */
export function StatusChip({ status }: { status: StepStatus | null }) {
  const tr = useTr()
  return (
    <HStack as="span" gap={4} className="flow-status" data-status={status ?? 'none'}>
      <Icon name={status ? STATUS_ICON[status] : 'circle.dashed'} size={11} />
      <Text size="caption">{tr(statusLabel(status))}</Text>
    </HStack>
  )
}

/** The week ahead as seven columns, a tick where the routine fires (OpenBot's routine card). */
function WeekStrip({ upcoming }: { upcoming: number[] }) {
  const tr = useTr()
  const start = new Date()
  start.setHours(0, 0, 0, 0)
  const days = Array.from({ length: 7 }, (_, index) => {
    const from = start.getTime() + index * DAY_MS
    const ticks = upcoming
      .filter((at) => at >= from && at < from + DAY_MS)
      .map((at) => ((at - from) / DAY_MS) * 100)
    return { from, ticks, today: index === 0 }
  })
  const count = days.reduce((total, day) => total + day.ticks.length, 0)
  if (count === 0) return null
  return (
    <div className="flow-week" role="img" aria-label={tr('Fires {count} times in the next seven days').replace('{count}', String(count))}>
      {days.map((day) => (
        <span key={day.from} className="flow-week-day" data-today={day.today ? 'true' : undefined}>
          <span className="flow-week-track">
            {day.ticks.map((left, index) => <span key={index} className="flow-week-tick" style={{ left: `${left}%` }} />)}
          </span>
        </span>
      ))}
    </div>
  )
}

/** The newest runs as dots, oldest on the left so the row reads forward in time. */
function RunDots({ runs }: { runs: FlowRoutineCard['recentRuns'] }) {
  const tr = useTr()
  const recent = runs.slice(0, 6).reverse()
  if (recent.length === 0) return null
  return (
    <span className="flow-runs" aria-label={tr('Recent runs')}>
      {recent.map((run) => (
        <span
          key={run.id}
          className="flow-run-dot"
          data-status={run.status}
          title={`${tr(statusLabel(run.status as StepStatus))} · ${new Date(run.createdAt).toLocaleString()}`}
        />
      ))}
    </span>
  )
}

interface Props {
  node: FlowNode
  /** Where the card stands now (a drag follows the pointer before it is saved). */
  x: number
  y: number
  bot?: Bot
  routine?: FlowRoutineCard
  /** The picture or clip step this card stands for. */
  media?: FlowMedia | null
  step: FlowStep | null
  unconnected: boolean
  selected: boolean
  connectTarget: boolean
  disabled: boolean
  /** The pointer that picked the card, so the canvas can add it to a multi-selection. */
  onSelect: (event: PointerEvent) => void
  onDragStart: (event: PointerEvent) => void
  onPortStart: (event: PointerEvent) => void
  onKey: (event: KeyboardEvent) => void
  /** Runs this routine's flow; only routine cards have it. */
  onRun?: () => void
  onStop?: () => void
  /** Opens this step's menu (Run from this step, Settings, Duplicate); not on routine cards. */
  onMenu?: (anchor: { x: number; top: number; bottom: number }) => void
  /** The custom name of this step, or '' when it has none. */
  stepName: string
  /** Clicking the title edits the name; absent on a routine card. */
  onRename?: (name: string) => void
  /** What the running step is doing, the step before it and how many steps are done. */
  progress?: { activity: string; previous: string; done: number; total: number } | null
  /** What this node's output port carries, and whether a line leaves it. */
  carry: 'text' | 'image' | 'file'
  connected: boolean
  /** Dimmed while the canvas search looks for another node. */
  matched: boolean
}

/** One node of the canvas: a routine with its week and last runs, a bot with its latest step, or a step that makes a picture or a clip. */
export function FlowNodeCard({ node, x, y, bot, routine, media, step, unconnected, selected, connectTarget, disabled, onSelect, onDragStart, onPortStart, onKey, onRun, onStop, onMenu, stepName, onRename, progress, carry, connected, matched }: Props) {
  const tr = useTr()
  const title = stepName || (node.kind === 'routine' ? (routine?.routine.name ?? '') : node.kind === 'media' ? tr(mediaTitle(media ?? null)) : (bot?.name ?? tr('Unknown bot')))
  const kindName = node.kind === 'routine' ? tr('Routine') : node.kind === 'media' ? tr(media?.kind === 'video' ? 'Clip step' : 'Picture step') : tr('Bot')
  const [editing, setEditing] = useState(false)
  const [draft, setDraft] = useState(stepName)
  useEffect(() => setDraft(stepName), [stepName])
  useEffect(() => { if (!onRename) setEditing(false) }, [onRename])
  const commit = () => {
    if (!onRename) return
    const next = draft.trim()
    if (next !== stepName) onRename(next)
    setEditing(false)
  }
  return (
    <div
      className="flow-node"
      data-kind={node.kind}
      data-selected={selected}
      data-target={connectTarget}
      data-matched={matched ? undefined : 'false'}
      role="button"
      tabIndex={0}
      aria-label={`${kindName}: ${title}`}
      aria-pressed={selected}
      style={{ transform: `translate(${x}px, ${y}px)`, ...(node.kind === 'routine' ? { '--flow-tint': routineTint(node.id) } as CSSProperties : {}) }}
      onPointerDown={(event) => {
        onSelect(event)
        if (!disabled) onDragStart(event)
      }}
      onKeyDown={onKey}
    >
      <HStack gap={10} align="center" className="flow-node-head">
        {node.kind === 'routine' ? (
          <Icon name="clock.arrow.circlepath" size={26} />
        ) : node.kind === 'media' ? (
          <Icon name={media?.kind === 'video' ? 'film' : 'sparkles'} size={24} />
        ) : bot ? (
          <BotAvatar bot={bot} size={30} animated={false} />
        ) : (
          <Icon name="questionmark.circle" size={26} />
        )}
        <VStack gap={2} shrink>
          {editing && onRename ? (
            <input
              className="routine-input flow-node-name"
              aria-label={tr('Step name')}
              placeholder={tr('Step name')}
              value={draft}
              maxLength={200}
              autoFocus
              onPointerDown={(event) => event.stopPropagation()}
              onKeyDown={(event) => {
                event.stopPropagation()
                if (event.key === 'Enter') commit()
                else if (event.key === 'Escape') setEditing(false)
              }}
              onBlur={commit}
            />
          ) : onRename ? (
            <button
              type="button"
              className="flow-node-title"
              title={tr('Click to rename this step')}
              aria-label={`${tr('Step name')}: ${title}. ${tr('Click to rename this step')}`}
              onPointerDown={(event) => event.stopPropagation()}
              onClick={(event) => { event.stopPropagation(); setDraft(stepName); setEditing(true) }}
            >
              <Text weight="semibold" truncate>{title}</Text>
            </button>
          ) : (
            <Text weight="semibold" truncate>{title}</Text>
          )}
          {node.kind === 'bot' ? (
            <Text size="caption" tone="secondary" truncate>{bot?.model || bot?.backend || ''}</Text>
          ) : node.kind === 'media' ? (
            <Text size="caption" tone="secondary" truncate>{tr(media?.kind === 'video' ? 'Clip' : 'Picture')}</Text>
          ) : (
            <Text size="caption" tone="secondary" truncate>{routine?.routine.triggerDescriptions.join(' · ')}</Text>
          )}
        </VStack>
        {node.kind !== 'routine' && onMenu ? (
          <button
            type="button"
            className="flow-node-menu-button"
            title={tr('Step menu')}
            aria-label={tr('Step menu')}
            disabled={disabled}
            onPointerDown={(event) => event.stopPropagation()}
            onClick={(event) => {
              event.stopPropagation()
              const rect = event.currentTarget.getBoundingClientRect()
              onMenu({ x: rect.left, top: rect.top, bottom: rect.bottom })
            }}
          >
            <Icon name="ellipsis" size={14} />
          </button>
        ) : null}
        {node.kind === 'routine' && routine?.routine.running && onStop ? (
          <button
            type="button"
            className="flow-run-button flow-stop-button"
            title={tr('Stop')}
            aria-label={tr('Stop')}
            disabled={disabled}
            onPointerDown={(event) => event.stopPropagation()}
            onClick={(event) => {
              event.stopPropagation()
              onStop()
            }}
          >
            <Icon name="stop.fill" size={12} />
          </button>
        ) : node.kind === 'routine' && onRun ? (
          <button
            type="button"
            className="flow-run-button"
            title={unconnected ? tr('Connect a bot first') : tr('Run this routine now')}
            aria-label={unconnected ? tr('Connect a bot first') : tr('Run this routine now')}
            disabled={disabled || routine?.routine.enabled === false || unconnected}
            onPointerDown={(event) => event.stopPropagation()}
            onClick={(event) => {
              event.stopPropagation()
              onRun()
            }}
          >
            <Icon name="play.fill" size={12} />
          </button>
        ) : null}
      </HStack>
      {node.kind === 'routine' ? (
        <>
          <WeekStrip upcoming={routine?.upcomingRuns ?? []} />
          <HStack gap={8} align="center" className="flow-node-foot">
            <RunDots runs={routine?.recentRuns ?? []} />
            <Text size="caption" tone="secondary" truncate>
              {unconnected ? tr('Not connected') : routine?.steps.length ? tr('Latest run') : tr('No runs yet')}
            </Text>
          </HStack>
        </>
      ) : (
        <VStack gap={4} className="flow-node-bottom">
          <StatusChip status={step?.status ?? null} />
          {progress ? (
            <div className="flow-node-progress" aria-live="polite">
              <Text size="caption" tone="secondary" truncate title={progress.activity}>
                {progress.activity || tr('Working…')}
              </Text>
              <Text size="caption" tone="secondary" truncate>
                {progress.previous ? `${tr('After')}: ${progress.previous}` : tr('First step')}
              </Text>
              <Text size="caption" tone="secondary">
                {tr('{done} of {total} steps done').replace('{done}', String(progress.done)).replace('{total}', String(progress.total))}
              </Text>
            </div>
          ) : null}
        </VStack>
      )}
      {node.kind !== 'routine' ? (
        <button
          type="button"
          className="flow-port"
          data-carry={carry}
          data-connected={connected ? 'true' : undefined}
          title={tr(node.kind === 'media' ? 'Drag to another bot or step to hand its file on' : 'Drag to another bot to hand its reply on')}
          aria-label={tr('Connect from this step')}
          disabled={disabled}
          onPointerDown={(event) => {
            event.stopPropagation()
            onPortStart(event)
          }}
        />
      ) : null}
    </div>
  )
}
