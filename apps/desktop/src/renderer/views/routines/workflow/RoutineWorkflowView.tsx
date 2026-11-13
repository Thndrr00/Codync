import { useEffect, useMemo, useRef, useState, type PointerEvent as ReactPointerEvent } from 'react'
import { isGroup, type Bot } from '@shared/models'
import { Button, IconButton, Spinner } from '../../../components/Controls'
import { BotAvatar } from '../../../components/Avatar'
import { AnchoredMenu, ModalHeader, Sheet } from '../../../components/Overlay'
import { Icon } from '../../../components/Icon'
import { HStack, Spacer, VStack } from '../../../components/Stack'
import { Text } from '../../../components/Text'
import { useModel } from '../../../lib/observable'
import type { FlowCanvas, FlowLink } from '../../../lib/routine-flow'
import { useTr } from '../../../lib/use-language'
import { useStore } from '../../../store/context'
import { errorText, type Routine, type RoutineRun } from '../routine'
import { RoutineEditorView } from '../RoutineEditorView'
import { RoutineRunHistory } from '../RoutineRunHistory'
import './workflow.css'

/** A routine of any bot, with what the list needs to show it. */
interface RoutineRow {
  botId: string
  botName: string
  routine: Routine
  runs: RoutineRun[]
}

/** One agent on the canvas: the routine's own bot or a bot a link reaches. */
interface CanvasAgent {
  botId: string
  name: string
  color: string
  shape: string
  model: string
  instruction: string
  link: FlowLink | null
}

interface Point {
  x: number
  y: number
}

/** A note or a text block someone drops on the canvas; kept beside the diagram, per routine. */
interface Mark {
  id: string
  kind: 'note' | 'text'
  x: number
  y: number
  text: string
}

const TRIGGER_WIDTH = 300
const AGENT_WIDTH = 340
const NODE_HEIGHT = 190
const COLUMN_GAP = 120

/** A schedule in plain words, from the host's own descriptions. */
function triggerLines(routine: Routine): string[] {
  const lines = routine.triggerDescriptions?.length ? routine.triggerDescriptions : routine.triggers.map((t) => t.type)
  return lines.length > 0 ? lines : ['Schedule']
}

function nextRunText(routine: Routine): string | null {
  const next = routine.nextRunAt ?? routine.nextRuns?.[0] ?? null
  if (!next) return null
  return new Date(next).toLocaleString(undefined, { weekday: 'short', day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' })
}

function runText(run: RoutineRun | undefined): string {
  if (!run) return 'No runs yet'
  const when = new Date(run.createdAt).toLocaleString(undefined, { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' })
  return `${run.status} · ${when}`
}

/** Every routine of every bot in this window, as the Routines page lists them. */
function useRoutineRows(): { rows: RoutineRow[]; loading: boolean; error: string | null; reload: () => void } {
  const store = useStore()
  useModel(store)
  const [rows, setRows] = useState<RoutineRow[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [rev, setRev] = useState(0)
  const load = () => setRev((n) => n + 1)

  useEffect(() => {
    let stopped = false
    const run = async () => {
      setLoading(true)
      try {
        const client = await store.ready()
        const bots = [...store.bots.values()].filter((bot) => !bot.hidden && !isGroup(bot) && bot.backend !== 'external')
        const listings = await Promise.all(
          bots.map(async (bot) => {
            const listing = await client.call<{ routines: Routine[]; runs: RoutineRun[] }>('routines', { botId: bot.id })
            return listing.routines.map((routine): RoutineRow => ({
              botId: bot.id,
              botName: bot.name,
              routine,
              runs: listing.runs.filter((r) => r.routineId === routine.id),
            }))
          }),
        )
        if (!stopped) {
          setRows(listings.flat().sort((a, b) => b.routine.updatedAt - a.routine.updatedAt))
          setError(null)
        }
      } catch (e) {
        if (!stopped) setError(errorText(e))
      } finally {
        if (!stopped) setLoading(false)
      }
    }
    void run()
    return () => {
      stopped = true
    }
  }, [store, rev])

  return { rows, loading, error, reload: load }
}

/** A routine's flow canvas: the trigger, its agents, and the toolbar OpenBot has. */
export function RoutineWorkflowView({ onClose }: { onClose?: () => void }) {
  const tr = useTr()
  const store = useStore()
  useModel(store)
  const { rows, loading, error, reload } = useRoutineRows()
  const [query, setQuery] = useState('')
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [flow, setFlow] = useState<FlowCanvas | null>(null)
  const [flowError, setFlowError] = useState<string | null>(null)
  const [positions, setPositions] = useState<Record<string, Point>>({})
  const [zoom, setZoom] = useState(1)
  const [tool, setTool] = useState<'select' | 'hand' | 'pen' | 'note' | 'shape' | 'text'>('select')
  const [menu, setMenu] = useState<{ x: number; y: number; kind: 'node' | 'routines' } | null>(null)
  const [ask, setAsk] = useState('')
  const [notice, setNotice] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [marks, setMarks] = useState<Mark[]>([])
  const [showDetails, setShowDetails] = useState(true)
  const [showHistory, setShowHistory] = useState(false)
  const [editing, setEditing] = useState(false)
  const viewport = useRef<HTMLDivElement | null>(null)
  const [pan, setPan] = useState<Point>({ x: 0, y: 0 })
  const drag = useRef<{ key: string; start: Point; origin: Point } | null>(null)
  const panning = useRef<{ start: Point; origin: Point } | null>(null)

  const selected = rows.find((row) => row.routine.id === selectedId) ?? null
  const shown = query.trim()
    ? rows.filter((row) => row.routine.name.toLowerCase().includes(query.trim().toLowerCase()) || row.botName.toLowerCase().includes(query.trim().toLowerCase()))
    : rows

  useEffect(() => {
    if (selectedId === null && rows.length > 0) setSelectedId(rows[0]!.routine.id)
  }, [rows, selectedId])

  // Notes and text blocks are the canvas's own layer: kept in this window, per routine.
  useEffect(() => {
    if (!selectedId) {
      setMarks([])
      return
    }
    try {
      const saved = localStorage.getItem(`codync.workflow.marks.${selectedId}`)
      setMarks(saved ? (JSON.parse(saved) as Mark[]) : [])
    } catch {
      setMarks([])
    }
  }, [selectedId])
  const saveMarks = (next: Mark[]) => {
    setMarks(next)
    if (!selectedId) return
    try {
      localStorage.setItem(`codync.workflow.marks.${selectedId}`, JSON.stringify(next))
    } catch {
      // A full quota only costs the annotation, not the flow.
    }
  }

  // The flow of the bot that owns the routine: its links are the agents the canvas draws.
  useEffect(() => {
    if (!selected) {
      setFlow(null)
      return
    }
    let stopped = false
    void (async () => {
      try {
        const client = await store.ready()
        const canvas = await client.call<FlowCanvas>('routineFlowCanvas', { botId: selected.botId })
        if (!stopped) {
          setFlow(canvas)
          setFlowError(null)
        }
      } catch (e) {
        if (!stopped) setFlowError(errorText(e))
      }
    })()
    return () => {
      stopped = true
    }
  }, [selected, store, busy])

  const bots = store.bots
  const agents = useMemo<CanvasAgent[]>(() => {
    if (!selected) return []
    const owner = bots.get(selected.botId)
    const list: CanvasAgent[] = [
      {
        botId: selected.botId,
        name: owner?.name ?? selected.botName,
        color: owner?.avatarColor ?? 'blue',
        shape: owner?.avatarShape ?? 'blob',
        model: owner?.model ?? 'Default model',
        instruction: selected.routine.instruction,
        link: null,
      },
    ]
    const links = flow?.links.filter((link) => link.routineId === selected.routine.id) ?? []
    for (const link of links) {
      if (list.some((agent) => agent.botId === link.toBotId)) continue
      const bot = bots.get(link.toBotId)
      list.push({
        botId: link.toBotId,
        name: bot?.name ?? link.toBotId,
        color: bot?.avatarColor ?? 'violet',
        shape: bot?.avatarShape ?? 'squircle',
        model: bot?.model ?? 'Default model',
        instruction: link.instruction,
        link,
      })
    }
    return list
  }, [selected, flow, bots])

  const addable = [...bots.values()].filter(
    (bot) => !bot.hidden && !isGroup(bot) && bot.backend !== 'external' && !agents.some((agent) => agent.botId === bot.id),
  )

  const steps = flow?.routines.find((card) => card.routine.id === selected?.routine.id)?.steps ?? []
  const stepLabel = (botId: string): string | null => {
    const step = steps.find((s) => s.botId === botId)
    if (!step) return null
    if (step.status === 'running') return tr('Running now')
    if (step.status === 'succeeded') return tr('Finished')
    if (step.status === 'failed') return step.error || tr('Failed')
    if (step.status === 'cancelled') return tr('Cancelled')
    return tr('Skipped')
  }

  const nodePoint = (key: string, index: number): Point =>
    positions[key] ?? { x: 40 + index * (AGENT_WIDTH + COLUMN_GAP), y: 90 }

  const runFlow = async () => {
    if (!selected) return
    setBusy(true)
    setNotice(null)
    try {
      const client = await store.ready()
      await client.call('runRoutine', { botId: selected.botId, id: selected.routine.id })
      setNotice(tr('The flow is running. Its steps show here as they finish.'))
      reload()
    } catch (e) {
      setNotice(errorText(e))
    }
    setBusy(false)
  }

  const saveInstruction = async (agent: CanvasAgent, text: string) => {
    if (!selected) return
    try {
      const client = await store.ready()
      if (agent.link) {
        await client.call('updateRoutineFlowLink', { linkId: agent.link.id, instruction: text })
        setNotice(tr('Saved.'))
        return
      }
      // The routine's own instruction changes through the bot's routine tools, as a person would ask.
      store.send(`In the routine "${selected.routine.name}", change its instruction to: ${text}`, selected.botId)
      setNotice(tr('Sent to the agent: it updates the routine with its tools.'))
    } catch (e) {
      setNotice(errorText(e))
    }
  }

  const connectAgent = async (botId: string) => {
    if (!selected) return
    try {
      const client = await store.ready()
      await client.call('connectRoutineFlow', { routineId: selected.routine.id, fromBotId: selected.botId, toBotId: botId, instruction: '' })
      setNotice(tr('Agent added to the diagram.'))
      reload()
    } catch (e) {
      setNotice(errorText(e))
    }
  }

  const askAgent = async () => {
    const text = ask.trim()
    if (!text || !selected) return
    try {
      await store.send(text, selected.botId)
      setAsk('')
      setNotice(tr('Sent to the agent. It edits the diagram with its tools.'))
    } catch (e) {
      setNotice(errorText(e))
    }
  }

  const zoomBy = (delta: number) => setZoom((z) => Math.min(1.8, Math.max(0.4, Number((z + delta).toFixed(2)))))
  const openMenu = (kind: 'node' | 'routines', event?: { clientX: number; clientY: number }) =>
    setMenu(event ? { x: event.clientX, y: event.clientY, kind } : { x: window.innerWidth / 2, y: window.innerHeight / 2, kind })

  const onPointerDown = (event: ReactPointerEvent) => {
    const target = event.target as HTMLElement
    if ((tool === 'note' || tool === 'text') && !target.closest('.workflow-node') && !target.closest('.workflow-mark')) {
      const rect = viewport.current?.getBoundingClientRect()
      if (rect) {
        const mark: Mark = {
          id: crypto.randomUUID(),
          kind: tool,
          x: (event.clientX - rect.left - pan.x) / zoom,
          y: (event.clientY - rect.top - pan.y) / zoom,
          text: '',
        }
        saveMarks([...marks, mark])
        setTool('select')
      }
      return
    }
    if (tool !== 'hand') return
    panning.current = { start: { x: event.clientX, y: event.clientY }, origin: pan }
    ;(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId)
  }
  const onPointerMove = (event: ReactPointerEvent) => {
    if (panning.current) {
      setPan({ x: panning.current.origin.x + event.clientX - panning.current.start.x, y: panning.current.origin.y + event.clientY - panning.current.start.y })
      return
    }
    const moving = drag.current
    if (!moving) return
    const dx = (event.clientX - moving.start.x) / zoom
    const dy = (event.clientY - moving.start.y) / zoom
    setPositions((current) => ({ ...current, [moving.key]: { x: moving.origin.x + dx, y: moving.origin.y + dy } }))
  }
  const endPointer = () => {
    panning.current = null
    drag.current = null
  }

  return (
    <div className="workflow-page">
      <header className="workflow-header">
        <HStack gap={6} align="center">
          <IconButton title={tr('Back')} icon="chevron.left" onClick={() => (selectedId ? setSelectedId(null) : onClose?.())} />
          <IconButton title={tr('Forward')} icon="chevron.right" />
          <button type="button" className="workflow-title" onClick={(e) => openMenu('routines', e)}>
            <Text weight="semibold">{selected?.routine.name ?? tr('Workflow without a title')}</Text>
            <Icon name="chevron.down" size={12} />
          </button>
        </HStack>
        <Spacer />
        <HStack gap={12} align="center">
          <IconButton title={tr('Routine details')} icon="sidebar.left" selected={showDetails} onClick={() => setShowDetails((v) => !v)} />
          <button type="button" className="workflow-history" onClick={() => setShowHistory(true)}>
            <Icon name="clock" size={14} />
            <Text size="caption">{tr('History')}</Text>
            <span className="workflow-count">{selected?.runs.length ?? 0}</span>
          </button>
          <Button disabled={!selected || busy} onClick={() => void runFlow()}>
            {busy ? <Spinner /> : <Icon name="play.circle" size={14} />}
            {tr('Run flow')}
          </Button>
        </HStack>
      </header>

      <div className={`workflow-body ${showDetails ? '' : 'no-details'}`}>
        <aside className="workflow-list">
          <label className="workflow-search">
            <Icon name="magnifyingglass" size={13} />
            <input value={query} onChange={(e) => setQuery(e.target.value)} placeholder={tr('Search')} />
          </label>
          {loading ? <HStack gap={8} className="workflow-loading"><Spinner /><Text size="caption" tone="secondary">{tr('Loading routines…')}</Text></HStack> : null}
          {error ? <Text size="caption" tone="danger">{error}</Text> : null}
          <div className="workflow-rows" role="list">
            {shown.map((row) => (
              <button
                key={row.routine.id}
                type="button"
                role="listitem"
                className="workflow-row"
                data-selected={row.routine.id === selectedId ? 'true' : undefined}
                onClick={() => setSelectedId(row.routine.id)}
              >
                <span className="workflow-row-avatar" data-paused={row.routine.enabled ? undefined : 'true'}>
                  {bots.get(row.botId) ? <BotAvatar bot={bots.get(row.botId)!} size={32} animated={false} /> : row.botName.slice(0, 1).toUpperCase()}
                </span>
                <span className="workflow-row-text">
                  <span className="workflow-row-top">
                    <strong>{row.routine.name}</strong>
                    <time>{new Date(row.routine.updatedAt).toLocaleDateString(undefined, { day: 'numeric', month: 'short' })}</time>
                  </span>
                  <span className="workflow-row-sub">{row.routine.instruction || row.botName}</span>
                </span>
              </button>
            ))}
            {!loading && shown.length === 0 ? <Text size="caption" tone="secondary" className="workflow-empty">{tr('No routines yet. Create one with a bot and it appears here.')}</Text> : null}
          </div>
          <div className="workflow-promo">
            <strong>{tr('Codync for your phone')}</strong>
            <p>{tr('The app is available for iPhone and Android.')}</p>
            <Button kind="secondary" onClick={() => window.codync.app.openExternal('https://codync.dev/download')}>{tr('How to install')}</Button>
          </div>
          {notice ? <p className="workflow-notice">{notice}</p> : null}
        </aside>

        <div
          ref={viewport}
          className={`workflow-canvas tool-${tool}`}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={endPointer}
          onPointerCancel={endPointer}
          onWheel={(e) => {
            if (!e.ctrlKey && !e.metaKey) return
            zoomBy(e.deltaY > 0 ? -0.08 : 0.08)
          }}
          onContextMenu={(e) => {
            e.preventDefault()
            openMenu('node', e)
          }}
        >
          <div
            className="workflow-world"
            style={{ transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})` }}
            onDoubleClick={(e) => openMenu('node', e)}
          >
            {selected && agents.length > 1 ? (
              <svg className="workflow-edges" aria-hidden>
                {agents.slice(1).map((agent, index) => {
                  const from = nodePoint('trigger', 0)
                  const to = nodePoint(`agent:${agent.botId}`, index + 1)
                  const x1 = from.x + TRIGGER_WIDTH
                  const y1 = from.y + 90
                  const x2 = to.x
                  const y2 = to.y + 95
                  const mid = (x1 + x2) / 2
                  return <path key={agent.botId} d={`M${x1},${y1} C${mid},${y1} ${mid},${y2} ${x2},${y2}`} />
                })}
              </svg>
            ) : null}
            {selected ? (
              <>
                <article
                  className="workflow-node trigger"
                  style={{ left: nodePoint('trigger', 0).x, top: nodePoint('trigger', 0).y, width: TRIGGER_WIDTH }}
                  onPointerDown={(e) => {
                    if (tool !== 'select') return
                    drag.current = { key: 'trigger', start: { x: e.clientX, y: e.clientY }, origin: nodePoint('trigger', 0) }
                    ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
                  }}
                >
                  <span className="workflow-node-kicker">{tr('Trigger')}</span>
                  <div className="workflow-node-head">
                    <span className="workflow-node-icon schedule"><Icon name="calendar" size={14} /></span>
                    <strong>{tr('Schedule')}</strong>
                    <Spacer />
                    <IconButton title={tr('Edit routine')} icon="pencil" size={22} onClick={() => setEditing(true)} />
                    <Icon name="chevron.down" size={12} />
                  </div>
                  <div className="workflow-node-body">
                    <div className="workflow-schedule-line">
                      {triggerLines(selected.routine).map((line) => (
                        <p key={line}>{line}</p>
                      ))}
                    </div>
                    <div className="workflow-next">
                      <span>{tr('Next run')}</span>
                      <strong>{nextRunText(selected.routine) ?? '—'}</strong>
                    </div>
                    <Button className="workflow-run-now" disabled={busy} onClick={() => void runFlow()}>
                      {tr('Run now')}
                    </Button>
                  </div>
                </article>

                {agents.map((agent, index) => {
                  const key = `agent:${agent.botId}`
                  const point = nodePoint(key, index + 1)
                  return (
                    <article
                      key={agent.botId}
                      className="workflow-node agent"
                      data-owner={agent.link === null ? 'true' : undefined}
                      style={{ left: point.x, top: point.y, width: AGENT_WIDTH }}
                      onPointerDown={(e) => {
                        if (tool !== 'select') return
                        drag.current = { key, start: { x: e.clientX, y: e.clientY }, origin: point }
                        ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
                      }}
                    >
                      <span className="workflow-node-kicker">{tr('Agent')}</span>
                      <textarea
                        className="workflow-node-task"
                        defaultValue={agent.instruction}
                        placeholder={tr('Tell the agent what to do')}
                        onBlur={(e) => void saveInstruction(agent, e.target.value)}
                      />
                      <div className="workflow-node-footer">
                        <span className="workflow-avatar tiny">
                          {bots.get(agent.botId) ? <BotAvatar bot={bots.get(agent.botId)!} size={18} animated={false} /> : agent.name.slice(0, 1).toUpperCase()}
                        </span>
                        <span className="workflow-agent-name">{agent.name}</span>
                        <Spacer />
                        <span className="workflow-model">
                          <Icon name="sparkles" size={12} />
                          {agent.model}
                        </span>
                      </div>
                      <div className="workflow-node-state" data-status={steps.find((s) => s.botId === agent.botId)?.status}>
                        {stepLabel(agent.botId) ?? tr('No runs yet')}
                        {selected.routine.enabled ? null : <span className="workflow-node-warn">{tr('Paused')}</span>}
                      </div>
                      <Button kind="secondary" className="workflow-run-step" disabled={busy} onClick={() => void runFlow()}>
                        {tr('Run step')}
                      </Button>
                    </article>
                  )
                })}
              </>
            ) : null}
            {marks.map((mark) => (
              <div
                key={mark.id}
                className={`workflow-mark ${mark.kind}`}
                style={{ left: mark.x, top: mark.y }}
                onPointerDown={(e) => e.stopPropagation()}
              >
                <textarea
                  value={mark.text}
                  autoFocus={mark.text === ''}
                  placeholder={mark.kind === 'note' ? tr('Sticky note') : tr('Text')}
                  onChange={(e) => saveMarks(marks.map((m) => (m.id === mark.id ? { ...m, text: e.target.value } : m)))}
                />
                <button type="button" className="workflow-mark-remove" aria-label={tr('Remove')} onClick={() => saveMarks(marks.filter((m) => m.id !== mark.id))}>
                  <Icon name="xmark" size={10} />
                </button>
              </div>
            ))}
          </div>

          <div className="workflow-tools" role="toolbar" aria-label={tr('Canvas tools')}>
            <div className="workflow-zoom">
              <IconButton title={tr('Zoom out')} icon="minus.circle" onClick={() => zoomBy(-0.1)} />
              <Text size="caption">{Math.round(zoom * 100)}%</Text>
              <IconButton title={tr('Zoom in')} icon="plus" onClick={() => zoomBy(0.1)} />
              <IconButton title={tr('Fit to window')} icon="square.grid.2x2" onClick={() => { setZoom(1); setPan({ x: 0, y: 0 }) }} />
            </div>
            <div className="workflow-tool-group">
              {([
                ['select', 'cursorarrow.motionlines', tr('Select')],
                ['hand', 'hand.raised', tr('Pan')],
                ['pen', 'pencil', tr('Draw')],
                ['note', 'doc.text', tr('Sticky note')],
                ['shape', 'square.on.square', tr('Shapes')],
                ['text', 'textformat', tr('Text')],
              ] as const).map(([id, icon, label]) => (
                <IconButton key={id} title={label} icon={icon} selected={tool === id} onClick={() => setTool(id)} />
              ))}
              <IconButton title={tr('Search')} icon="magnifyingglass" onClick={(e) => openMenu('node', e)} />
            </div>
            <button type="button" className="workflow-add styled-button primary" onClick={(e) => openMenu('node', e)}>
              <Icon name="plus" size={12} />
              {tr('Add node')}
            </button>
          </div>

          <div className="workflow-ask">
            <span className="workflow-avatar tiny">
              {selected && bots.get(selected.botId) ? <BotAvatar bot={bots.get(selected.botId)!} size={18} animated={false} /> : (selected?.botName ?? 'A').slice(0, 1).toUpperCase()}
            </span>
            <input
              value={ask}
              onChange={(e) => setAsk(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') void askAgent()
              }}
              placeholder={tr('Ask an agent to edit this diagram')}
              disabled={!selected}
            />
            <IconButton title={tr('Send')} icon="arrow.up" onClick={() => void askAgent()} />
          </div>

          <AnchoredMenu
            open={menu !== null}
            onClose={() => setMenu(null)}
            point={menu}
            items={() =>
              menu?.kind === 'routines'
                ? rows.map((row) => ({
                    title: `${row.routine.name} · ${row.botName}`,
                    action: () => {
                      setMenu(null)
                      setSelectedId(row.routine.id)
                    },
                  }))
                : addable.length === 0
                  ? [{ title: tr('Every bot is already here'), action: () => setMenu(null) }]
                  : addable.map((bot: Bot) => ({
                      title: `${tr('Add')} ${bot.name}`,
                      action: () => {
                        setMenu(null)
                        void connectAgent(bot.id)
                      },
                    }))
            }
          />
        </div>

        <aside className="workflow-details" style={showDetails ? undefined : { display: 'none' }}>
          <strong>{tr('Routine details')}</strong>
          {selected ? (
            <VStack gap={10}>
              <p className="workflow-detail-lede">{tr('Run the diagram to see what each agent receives and returns.')}</p>
              <dl className="workflow-facts">
                <dt>{tr('Bot')}</dt><dd>{selected.botName}</dd>
                <dt>{tr('Trigger')}</dt><dd>{triggerLines(selected.routine).join(' · ')}</dd>
                <dt>{tr('Next run')}</dt><dd>{nextRunText(selected.routine) ?? '—'}</dd>
                <dt>{tr('Agents')}</dt><dd>{agents.length}</dd>
                <dt>{tr('Last run')}</dt><dd>{runText(selected.runs[0])}</dd>
              </dl>
              {flowError ? <Text size="caption" tone="danger">{flowError}</Text> : null}
            </VStack>
          ) : (
            <p className="workflow-detail-lede">{tr('Pick a routine to see its diagram.')}</p>
          )}
        </aside>
      </div>

      <Sheet open={showHistory} onClose={() => setShowHistory(false)} width={520} height={Math.min(620, window.innerHeight - 80)}>
        <VStack gap={12} className="workflow-sheet">
          <ModalHeader icon="clock" title={tr('History')} onClose={() => setShowHistory(false)} />
          {selected ? <RoutineRunHistory runs={selected.runs} name={selected.routine.name} /> : <Text tone="secondary">{tr('Pick a routine to see its history.')}</Text>}
        </VStack>
      </Sheet>

      <Sheet open={editing} onClose={() => setEditing(false)} width={560} height={Math.min(680, window.innerHeight - 60)}>
        {selected ? (
          <VStack gap={12} className="workflow-sheet">
            <ModalHeader icon="pencil" title={tr('Edit routine')} onClose={() => setEditing(false)} />
            <RoutineEditorView
              botId={selected.botId}
              routine={selected.routine}
              saved={() => {
                setEditing(false)
                reload()
              }}
            />
          </VStack>
        ) : null}
      </Sheet>
    </div>
  )
}
