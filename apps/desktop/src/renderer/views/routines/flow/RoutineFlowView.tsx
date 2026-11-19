import { useEffect, useMemo, useState } from 'react'
import type { MediaKind } from '@shared/media'
import { isGroup, type Bot } from '@shared/models'
import { Button, ChoicePicker, DropdownMenu, IconButton, SegmentedChoice, Spinner } from '../../../components/Controls'
import { Dialog, DialogField, ModalHeader, Sheet } from '../../../components/Overlay'
import { AnchoredMenu } from '../../../components/Menu'
import { HStack, Spacer, VStack } from '../../../components/Stack'
import { Text } from '../../../components/Text'
import { useModel } from '../../../lib/observable'
import {
  NODE_WIDTH, canvasBotIds, connectProblem, layoutNodes, mediaKey, mediaTitle,
  type FlowCanvas, type FlowLink, type FlowMedia,
} from '../../../lib/routine-flow.ts'
import { useTr } from '../../../lib/use-language'
import { useStore } from '../../../store/context'
import { RoutineFlowStore } from '../../../store/routine-flow-store'
import { FlowCanvasView, type FlowSelection } from './FlowCanvas'
import {
  documentFromCanvas, type FlowChoice, type FlowConflict, type FlowDocument, type FlowSaveAnswer,
} from './flow-merge'
import { groupsOf, groupSpot, newGroupId, outputType, parsePort, type CanvasWithGroups, type FlowGroup } from './flow-groups'
import { FlowInspector, type FlowTarget, type MediaSettings } from './FlowInspector'
import { FlowList } from './FlowList'
import { RoutineEditorView } from '../RoutineEditorView'
import './routine-flow.css'

type Mode = 'canvas' | 'list'

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
  /** Someone else's edit was folded into ours without a choice being needed. */
  const [mergeNote, setMergeNote] = useState(false)
  /** A save found fields both people changed: the merged document and the choices still needed. */
  const [conflict, setConflict] = useState<{ document: FlowDocument; conflicts: FlowConflict[] } | null>(null)
  const [choices, setChoices] = useState<Record<string, FlowChoice>>({})
  const [creating, setCreating] = useState(false)
  const [choose, setChoose] = useState<{ from: string; to: string; at: { x: number; y: number }; routines: { id: string; name: string }[] } | null>(null)
  /** Keys being grouped, waiting for a name (Ctrl/Cmd+G). */
  const [groupDraft, setGroupDraft] = useState<string[] | null>(null)
  const [groupName, setGroupName] = useState('')
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

  const canvas = model.canvas as CanvasWithGroups | null
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
    // A line from a group's output port reaches every member of that output kind.
    const port = parsePort(from)
    if (port) {
      const group = groupsOf(canvas).find((g) => g.id === port.id)
      for (const key of group?.members ?? []) {
        if (outputType(canvas, key) !== port.type) continue
        const id = key.startsWith('media:') ? key : key.slice(key.indexOf(':') + 1)
        const options = connectable(canvas, id, to)
        if (options.length > 0) void connect(options[0].routine.id, id, to)
      }
      return
    }
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

  const targets = (from: string): FlowTarget[] => {
    if (!canvas) return []
    const bots: FlowTarget[] = roster
      .filter((bot) => bot.id !== from && connectable(canvas, from, bot.id).length > 0)
      .map((bot) => ({ id: bot.id, name: bot.name }))
    const steps: FlowTarget[] = canvas.mediaNodes
      .filter((media) => mediaKey(media.id) !== from && connectable(canvas, from, mediaKey(media.id)).length > 0)
      .map((media) => ({ id: mediaKey(media.id), name: mediaTitle(media) }))
    return [...bots, ...steps]
  }

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
  const addMedia = async (routineId: string, kind: MediaKind) => {
    setNote(null)
    const media = await flow.addMedia(routineId, kind)
    if (media) setSelection({ kind: 'node', key: mediaKey(media.id) })
  }
  const saveMedia = (media: FlowMedia, settings: MediaSettings) => void flow.updateMedia(media.id, settings)
  const removeMedia = async (media: FlowMedia) => {
    setSelection(null)
    await flow.removeMedia(media.id)
  }

  const fieldLabel = (field: string) => tr(
    field === 'nodes' ? 'Node' : field === 'links' ? 'Link' : field === 'settings' ? 'Step settings' : field === 'groups' ? 'Group' : field,
  )
  const versionText = (value: Record<string, unknown> | null) => value === null ? tr('Removed') : JSON.stringify(value, null, 2)
  const conflictId = (item: FlowConflict) => `${item.field}:${item.key}`

  /** Sends one choice at a time; the last one saves the whole merged document. */
  const applyChoices = async (waiting: { document: FlowDocument; conflicts: FlowConflict[] }) => {
    try {
      const client = await store.ready()
      let answer: FlowSaveAnswer | null = null
      for (const item of waiting.conflicts) {
        answer = await client.call<FlowSaveAnswer>('resolveRoutineFlowConflict', {
          botId, field: item.field, key: item.key, choice: choices[conflictId(item)] ?? 'local',
        })
        if (answer.saved) break
      }
      if (answer?.saved) {
        setConflict(null)
        await flow.load()
      }
    } catch {
      setNote(tr('Could not save the group.'))
    }
  }

  // Groups are drawing-only data on the host's canvas. The save carries the whole flow document
  // with the revision we started from, so the host can merge someone else's edits field by field
  // and writes the merged groups back to the canvas in the same transaction.
  const saveGroups = async (next: FlowGroup[]) => {
    if (!canvas || offline) return
    setMergeNote(false)
    try {
      const answer = await (await store.ready()).call<FlowSaveAnswer>('saveRoutineFlow', {
        botId, baseRev: canvas.rev, document: documentFromCanvas(canvas, next),
      })
      if (answer.conflicts?.length) {
        setConflict({ document: answer.document, conflicts: answer.conflicts })
        setChoices({})
        return
      }
      if (answer.merged) setMergeNote(true)
    } catch {
      setNote(tr('Could not save the group.'))
    }
    await flow.load()
  }
  const createGroup = async (keys: string[], name: string) => {
    if (!canvas) return
    const group: FlowGroup = {
      id: newGroupId(groupsOf(canvas)),
      name: name.trim() || tr('New group'),
      members: keys,
      collapsed: false,
      ...groupSpot(layoutNodes(canvas), keys),
    }
    await saveGroups([...groupsOf(canvas), group])
    setSelection({ kind: 'group', id: group.id })
  }
  const ungroup = async (id: string) => {
    if (!canvas) return
    if (selection?.kind === 'group' && selection.id === id) setSelection(null)
    await saveGroups(groupsOf(canvas).filter((g) => g.id !== id))
  }
  const toggleGroup = async (id: string) => {
    if (!canvas) return
    await saveGroups(groupsOf(canvas).map((g) => (g.id === id ? { ...g, collapsed: !g.collapsed } : g)))
  }
  const renameGroup = async (id: string, name: string) => {
    if (!canvas) return
    await saveGroups(groupsOf(canvas).map((g) => (g.id === id ? { ...g, name } : g)))
  }
  const moveGroup = async (id: string, x: number, y: number) => {
    if (!canvas) return
    const group = groupsOf(canvas).find((g) => g.id === id)
    if (!group) return
    const dx = x - group.x
    const dy = y - group.y
    const places = new Map(layoutNodes(canvas).map((node) => [node.key, node]))
    for (const key of group.members) {
      const node = places.get(key)
      if (node) await flow.move(key, node.x + dx, node.y + dy)
    }
    await saveGroups(groupsOf(canvas).map((g) => (g.id === id ? { ...g, x, y } : g)))
  }
  /** A paste or drop of two or more files makes one picture step per file, all in one group. */
  const addFiles = async (files: { name: string }[]) => {
    if (!canvas || offline) return
    const routine = canvas.routines.find((r) => r.routine.botId === botId) ?? canvas.routines[0]
    if (!routine) {
      setNote(tr('Add a routine before dropping files.'))
      return
    }
    const members: string[] = []
    for (const file of files) {
      const media = await flow.addMedia(routine.routine.id, 'image')
      if (!media) return
      await flow.updateMedia(media.id, { prompt: file.name })
      members.push(mediaKey(media.id))
    }
    if (members.length < 2) return
    const spot = freeSpot(canvas)
    await saveGroups([...groupsOf(canvas), {
      id: newGroupId(groupsOf(canvas)), name: tr('New group'), members, collapsed: false, x: spot.x, y: spot.y,
    }])
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
      {mergeNote ? <Text size="caption" tone="attention" className="flow-merge-note">{tr('Someone else edited this flow; their changes were merged in.')}</Text> : null}
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
                onMoveGroup={(id, x, y) => void moveGroup(id, x, y)}
                onConnect={dropped}
                onGroup={(keys) => { setGroupDraft(keys); setGroupName('') }}
                onUngroup={(id) => void ungroup(id)}
                onToggleGroup={(id) => void toggleGroup(id)}
                onFiles={(files) => void addFiles(files)}
                onRun={(id) => {
                  setSelection({ kind: 'node', key: `routine:${id}` })
                  void flow.test(id)
                }}
                onStop={() => void store.stop(botId)}
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
            onAddMedia={(id, kind) => void addMedia(id, kind)}
            onSaveMedia={saveMedia}
            onRemoveMedia={(media) => void removeMedia(media)}
            onRenameGroup={(id, name) => void renameGroup(id, name)}
            onToggleGroup={(id) => void toggleGroup(id)}
            onUngroup={(id) => void ungroup(id)}
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
      <Dialog
        open={groupDraft !== null}
        title={tr('Name the group')}
        actions={[{
          title: tr('Group'),
          action: () => {
            if (groupDraft) void createGroup(groupDraft, groupName)
            setGroupDraft(null)
          },
        }]}
        onClose={() => setGroupDraft(null)}
      >
        <DialogField
          value={groupName}
          onChange={setGroupName}
          placeholder={tr('Group name')}
          label={tr('Group name')}
          onSubmit={() => {
            if (groupDraft) void createGroup(groupDraft, groupName)
            setGroupDraft(null)
          }}
        />
      </Dialog>
      <Dialog
        open={conflict !== null}
        title={tr('This flow changed while you were editing it.')}
        message={tr('Choose which version to keep for each item, then apply.')}
        actions={[{
          title: tr('Apply choices'),
          action: () => { if (conflict) void applyChoices(conflict) },
        }]}
        onClose={() => setConflict(null)}
      >
        {conflict ? (
          <VStack gap={10} className="flow-conflicts">
            {conflict.conflicts.map((item) => {
              const id = conflictId(item)
              return (
                <VStack gap={4} key={id} className="flow-conflict">
                  <Text size="callout" weight="semibold">{fieldLabel(item.field)} · {item.key}</Text>
                  <HStack gap={6} align="center">
                    <Text size="caption" tone="secondary" className="flow-conflict-label">{tr('Your version')}</Text>
                    <Text size="caption" lines={2} grow title={versionText(item.local)} className="flow-conflict-text">{versionText(item.local)}</Text>
                  </HStack>
                  <HStack gap={6} align="center">
                    <Text size="caption" tone="secondary" className="flow-conflict-label">{tr('Their version')}</Text>
                    <Text size="caption" lines={2} grow title={versionText(item.incoming)} className="flow-conflict-text">{versionText(item.incoming)}</Text>
                  </HStack>
                  <ChoicePicker<FlowChoice>
                    fits
                    selection={choices[id] ?? 'local'}
                    options={[{ id: 'local', label: 'Your version' }, { id: 'incoming', label: 'Their version' }]}
                    onChange={(choice) => setChoices({ ...choices, [id]: choice })}
                  />
                </VStack>
              )
            })}
          </VStack>
        ) : null}
      </Dialog>
    </VStack>
  )
}
