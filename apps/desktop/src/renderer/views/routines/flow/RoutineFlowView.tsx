import { useEffect, useMemo, useRef, useState } from 'react'
import { isGroup, type Bot } from '@shared/models'
import { Button, DropdownMenu, IconButton, SegmentedChoice, Spinner } from '../../../components/Controls'
import { ModalHeader, Sheet } from '../../../components/Overlay'
import { AnchoredMenu } from '../../../components/Menu'
import { HStack, Spacer, VStack } from '../../../components/Stack'
import { Text } from '../../../components/Text'
import { useModel } from '../../../lib/observable'
import { NODE_WIDTH, canvasBotIds, connectProblem, layoutNodes, type FlowCanvas, type FlowLink } from '../../../lib/routine-flow.ts'
import { useTr } from '../../../lib/use-language'
import { useStore } from '../../../store/context'
import { RoutineFlowStore } from '../../../store/routine-flow-store'
import { FlowCanvasView, type FlowSelection } from './FlowCanvas'
import { FlowInspector } from './FlowInspector'
import { FlowList } from './FlowList'
import { RoutineEditorView } from '../RoutineEditorView'
import './routine-flow.css'

type Mode = 'canvas' | 'list'

/** A selection as the view's state stores it (card K14 keeps each agent's lane here too). */
function selectionState(selection: FlowSelection) {
  if (!selection) return null
  if (selection.kind === 'node') return { kind: 'node', key: selection.key }
  if (selection.kind === 'edge') return { kind: 'edge', id: selection.id }
  return { kind: 'lane', routineId: selection.routineId, botId: selection.botId }
}

/** The bots a routine flow can hand work to: agents of their own, not hidden. */
const canTakeWork = (bot: Bot) => !bot.hidden && !isGroup(bot) && bot.backend !== 'external'

/** Where a bot added by hand lands: right of everything on the canvas. */
function freeSpot(canvas: FlowCanvas) {
  const nodes = layoutNodes(canvas)
  return { x: Math.max(0, ...nodes.map((n) => n.x + NODE_WIDTH + 90)), y: Math.min(0, ...nodes.map((n) => n.y)) }
}

/** A bot's routine flow: its canvas (or the same flow as a list), and an inspector for the selection. */
export function RoutineFlowView({ botId, onExpand }: { botId: string; onExpand?: () => void }) {
  const tr = useTr()
  const store = useStore()
  const [flow] = useState(() => new RoutineFlowStore(botId, () => store.ready()))
  const model = useModel(flow)
  const [mode, setMode] = useState<Mode>('canvas')
  const [selection, setSelection] = useState<FlowSelection>(null)
  const [note, setNote] = useState<string | null>(null)
  const [creating, setCreating] = useState(false)
  const [choose, setChoose] = useState<{ from: string; to: string; at: { x: number; y: number }; routines: { id: string; name: string }[] } | null>(null)
  /** The saved view state is read before this window reports its own, so a reload keeps the lane. */
  const [restored, setRestored] = useState(false)
  const offline = store.isOffline
  const rev = store.flowRev(botId)

  useEffect(() => {
    flow.resume()
    void flow.load()
    return () => flow.dispose()
  }, [flow])
  // The host says the canvas changed (another client, a step finishing): load it again.
  useEffect(() => {
    if (rev > 0) void flow.load()
  }, [rev, flow])
  // Card K4/K14: a reload restores the mode and the lane the person (or an agent) focused.
  useEffect(() => {
    let live = true
    void store.readViewState(`routine:${botId}`).then((state) => {
      if (!live) return
      const picked = state?.selected as { kind?: unknown; id?: unknown; routineId?: unknown; botId?: unknown } | null | undefined
      if (picked?.kind === 'lane' && typeof picked.routineId === 'string' && typeof picked.botId === 'string') {
        setSelection({ kind: 'lane', routineId: picked.routineId, botId: picked.botId })
      } else if (picked?.kind === 'routine' && typeof picked.id === 'string') {
        setSelection({ kind: 'node', key: `routine:${picked.id}` })
      }
      const savedMode = state?.mode
      if (savedMode === 'list' || savedMode === 'canvas') setMode(savedMode)
      setRestored(true)
    })
    return () => {
      live = false
    }
  }, [store, botId])
  // Card K4: the canvas reports what the person sees, and follows the agent's named actions.
  useEffect(() => {
    if (!restored) return
    store.reportView(`routine:${botId}`, [
      { op: 'set', path: '/open', value: true },
      { op: 'set', path: '/mode', value: mode },
      { op: 'set', path: '/selected', value: selectionState(selection) },
    ])
  }, [store, botId, mode, selection, restored])
  useEffect(() => () => store.reportView(`routine:${botId}`, [{ op: 'set', path: '/open', value: false }]), [store, botId])
  const viewAction = store.lastViewAction
  const handledViewAction = useRef(store.lastViewAction?.seq ?? 0)
  useEffect(() => {
    if (!viewAction || viewAction.seq <= handledViewAction.current || viewAction.view !== `routine:${botId}`) return
    handledViewAction.current = viewAction.seq
    if (viewAction.action === 'focus_lane') {
      const { routineId, botId: laneBot } = viewAction.arguments
      if (typeof routineId !== 'string' || typeof laneBot !== 'string') return
      setMode('canvas')
      setSelection({ kind: 'lane', routineId, botId: laneBot })
      void flow.load()
      return
    }
    if (viewAction.action !== 'create_routine' && viewAction.action !== 'select_routine') return
    const routineId = viewAction.arguments.routineId
    if (typeof routineId !== 'string') return
    setMode('canvas')
    setSelection({ kind: 'node', key: `routine:${routineId}` })
    void flow.load()
  }, [viewAction, botId, flow])

  const canvas = model.canvas
  const bots = store.bots
  const roster = useMemo(() => [...bots.values()].filter(canTakeWork), [bots])

  const connectable = (c: FlowCanvas, from: string, to: string) =>
    c.routines.filter((r) => connectProblem(c, r.routine.id, from, to) === null)

  const connect = async (routineId: string, from: string, to: string) => {
    setNote(null)
    const link = await flow.connect(routineId, from, to, '')
    if (link) setSelection({ kind: 'edge', id: link.id })
  }

  const dropped = (from: string, to: string, at: { x: number; y: number }) => {
    if (!canvas || offline) return
    const routines = connectable(canvas, from, to)
    if (routines.length === 0) {
      const reason = canvas.routines.map((r) => connectProblem(canvas, r.routine.id, from, to)).find(Boolean)
      setNote(tr(reason ?? 'These bots can\'t be connected.'))
    } else if (routines.length === 1) {
      void connect(routines[0].routine.id, from, to)
    } else {
      setChoose({ from, to, at, routines: routines.map((r) => ({ id: r.routine.id, name: r.routine.name })) })
    }
  }

  const targets = (from: string): Bot[] =>
    canvas ? roster.filter((bot) => bot.id !== from && connectable(canvas, from, bot.id).length > 0) : []

  if (flow.unsupported) {
    return <Text size="callout" tone="secondary" className="outlined-card flow-state">{tr('This computer\'s host doesn\'t support routine flows yet. Update Codync on it.')}</Text>
  }
  if (!canvas) {
    return model.error ? (
      <VStack gap={8} className="outlined-card flow-state">
        <Text size="callout" tone="danger">{model.error}</Text>
        <Button kind="secondary" onClick={() => void flow.load()}>{tr('Try again')}</Button>
      </VStack>
    ) : (
      <HStack gap={8} className="outlined-card flow-state"><Spinner /><Text size="callout" tone="secondary">{tr('Loading the flow…')}</Text></HStack>
    )
  }

  const onCanvas = new Set(canvasBotIds(canvas))
  const addable = roster.filter((bot) => !onCanvas.has(bot.id))
  const addBot = (bot: Bot) => {
    const spot = freeSpot(canvas)
    void flow.place(bot.id, spot.x, spot.y)
  }
  const saveLink = (link: FlowLink, text: string, model: string | null, modelOptions: Record<string, string>) =>
    void flow.updateLink(link.id, text, model, modelOptions)
  const disconnect = async (link: FlowLink) => {
    setSelection(null)
    await flow.disconnect(link.id)
  }

  return (
    <VStack gap={8} className="routine-flow">
      <HStack gap={8} wrap>
        <SegmentedChoice selection={mode} options={[{ id: 'canvas', label: 'Canvas' }, { id: 'list', label: 'List' }]} onChange={setMode} />
        <Button kind="secondary" disabled={offline} onClick={() => setCreating(true)}>{tr('New routine')}</Button>
        <DropdownMenu
          title={tr('Add a bot to the canvas')}
          className="flow-menu-button"
          items={() => addable.map((bot) => ({ title: bot.name, action: () => addBot(bot) }))}
        >
          <Text>{addable.length === 0 ? tr('Every bot is on the canvas') : tr('Add a bot')}</Text>
        </DropdownMenu>
        <Spacer />
        {onExpand ? <IconButton title={tr('Open the canvas in a larger window')} icon="arrow.up.left.and.arrow.down.right" onClick={onExpand} /> : null}
        {model.busy || model.loading ? <Spinner /> : null}
      </HStack>
      {offline ? <Text size="caption" tone="attention">{tr('Offline: the flow is read-only until the computer is reachable.')}</Text> : null}
      {model.error ? <Text size="caption" tone="danger">{model.error}</Text> : null}
      {note ? <Text size="caption" tone="danger">{note}</Text> : null}
      {canvas.routines.length === 0 ? (
        <Text as="div" size="callout" tone="secondary" className="outlined-card flow-state">{tr('No routines yet. Ask the bot for one, then connect it to other bots here.')}</Text>
      ) : (
        <HStack gap={8} align="stretch" className="flow-body">
          <div className="flow-main">
            {mode === 'canvas' ? (
              <FlowCanvasView
                canvas={canvas}
                bots={bots}
                selection={selection}
                disabled={offline || model.busy}
                onSelect={setSelection}
                onMove={(key, x, y) => void flow.move(key, x, y)}
                onConnect={dropped}
                onRun={(id) => {
                  setSelection({ kind: 'node', key: `routine:${id}` })
                  void flow.test(id)
                }}
              />
            ) : (
              <FlowList canvas={canvas} bots={bots} selection={selection} onSelect={setSelection} />
            )}
          </div>
          <FlowInspector
            canvas={canvas}
            bots={bots}
            selection={selection}
            busy={model.busy}
            offline={offline}
            targets={targets}
            onSaveInstruction={saveLink}
            onDisconnect={(link) => void disconnect(link)}
            onTest={(id) => void flow.test(id)}
            onSetEnabled={(id, on) => void flow.setEnabled(id, on)}
            // No pointer position here: a routine choice, if needed, opens in the middle of the window.
            onConnectFrom={(from, to) => dropped(from, to, { x: window.innerWidth / 2, y: window.innerHeight / 2 })}
            onRemoveNode={(key) => void flow.removeNode(key)}
          />
        </HStack>
      )}
      <AnchoredMenu
        open={choose !== null}
        onClose={() => setChoose(null)}
        point={choose?.at ?? null}
        items={() => (choose?.routines ?? []).map((r) => ({ title: r.name, action: () => void connect(r.id, choose!.from, choose!.to) }))}
      />
      <Sheet open={creating} onClose={() => setCreating(false)} width={560} height={620}>
        {creating ? (
          <VStack gap={14} className="routine-flow-sheet">
            <ModalHeader icon="plus" title={tr('New routine')} onClose={() => setCreating(false)} />
            <div className="flow-new-routine">
              <RoutineEditorView
                botId={botId}
                routine={null}
                saved={(routine) => {
                  setCreating(false)
                  if (routine) setSelection({ kind: 'node', key: `routine:${routine.id}` })
                  void flow.load()
                }}
              />
            </div>
          </VStack>
        ) : null}
      </Sheet>
    </VStack>
  )
}
