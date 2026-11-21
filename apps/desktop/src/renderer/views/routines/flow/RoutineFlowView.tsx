import { useEffect, useMemo, useState } from 'react'
import type { MediaKind } from '@shared/media'
import { isGroup, type Bot } from '@shared/models'
import { Button, ChoicePicker, DropdownMenu, IconButton, SegmentedChoice, Spinner } from '../../../components/Controls'
import { Dialog, DialogField, ModalHeader, Sheet } from '../../../components/Overlay'
import { AnchoredMenu, type MenuItem } from '../../../components/Menu'
import { HStack, Spacer, VStack } from '../../../components/Stack'
import { Text } from '../../../components/Text'
import { useModel } from '../../../lib/observable'
import {
  NODE_WIDTH, botKey, canvasBotIds, connectProblem, layoutNodes, mediaEnd, mediaKey, mediaTitle, stepNameOf,
  type FlowCanvas, type FlowLink, type FlowMedia, type FlowRoutineCard, type FlowStep,
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
import { FlowEmptyState } from './FlowEmptyState'
import { FlowList } from './FlowList'
import { clipped, hasMore, leftmostNode, settingsOf, type FlowStepSettings } from './flow-settings'
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

/** One saved piece of a step's settings, with a copy button and Show more for long text. */
function SettingsText({ label, value }: { label: string; value: string }) {
  const tr = useTr()
  const [expanded, setExpanded] = useState(false)
  const [copied, setCopied] = useState(false)
  return (
    <VStack gap={2} className="flow-setting-text">
      <HStack gap={6} align="center">
        <Text size="caption" tone="secondary" grow>{tr(label)}</Text>
        <IconButton
          title={tr(copied ? 'Copied' : 'Copy')}
          icon={copied ? 'checkmark' : 'doc.on.doc'}
          onClick={() => {
            void navigator.clipboard.writeText(value).then(() => setCopied(true))
          }}
        />
      </HStack>
      <Text size="callout" className={expanded ? '' : 'flow-clamp'}>{clipped(value, expanded)}</Text>
      {hasMore(value) ? (
        <Button kind="secondary" onClick={() => setExpanded(!expanded)}>{tr(expanded ? 'Show less' : 'Show more')}</Button>
      ) : null}
    </VStack>
  )
}

/** The settings a step ran with, read from the run record; an older run simply has none. */
function SavedSettings({ settings }: { settings: FlowStepSettings | null }) {
  const tr = useTr()
  if (!settings) {
    return <Text size="callout" tone="secondary">{tr('This run was saved before steps kept their settings.')}</Text>
  }
  return (
    <VStack gap={10}>
      <Text size="caption" tone="secondary" weight="semibold">{tr('Settings used')}</Text>
      {settings.kind === 'bot' ? (
        <>
          {settings.instruction ? <SettingsText label="Instruction" value={settings.instruction} /> : null}
          {settings.model ? <SettingsText label="Model" value={settings.model} /> : null}
          {Object.entries(settings.modelOptions ?? {}).map(([key, value]) => (
            <SettingsText key={key} label={key} value={value} />
          ))}
        </>
      ) : (
        <>
          {settings.prompt ? <SettingsText label="What to make" value={settings.prompt} /> : null}
          {settings.model ? <SettingsText label="Model" value={settings.model} /> : null}
          {settings.aspect ? <SettingsText label="Aspect ratio" value={settings.aspect} /> : null}
          {settings.duration !== null && settings.duration !== undefined ? (
            <SettingsText label="Length" value={`${settings.duration}s`} />
          ) : null}
          {settings.resolution ? <SettingsText label="Resolution" value={settings.resolution} /> : null}
          {settings.transparent !== null && settings.transparent !== undefined ? (
            <SettingsText label="Transparent background" value={settings.transparent ? 'Yes' : 'No'} />
          ) : null}
          {settings.voice ? <SettingsText label="Voice" value={settings.voice} /> : null}
        </>
      )}
      <Text size="caption" tone="secondary" weight="semibold">{tr('Inputs read')}</Text>
      {settings.inputs.length === 0 ? (
        <Text size="callout" tone="secondary">{tr('No earlier step handed anything to this one.')}</Text>
      ) : (
        settings.inputs.map((input, index) => (
          <VStack key={`${input.from}-${index}`} gap={2} className="flow-setting-input">
            <Text size="caption" tone="secondary">{input.from}</Text>
            {input.instruction ? <SettingsText label="Instruction" value={input.instruction} /> : null}
            <SettingsText label="Answer" value={input.text} />
          </VStack>
        ))
      )}
    </VStack>
  )
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
  /** The step whose saved settings are open in the sheet. */
  const [shownSettings, setShownSettings] = useState<{ card: FlowRoutineCard; step: FlowStep } | null>(null)
  const [creating, setCreating] = useState(false)
  const [choose, setChoose] = useState<{ from: string; to: string; at: { x: number; y: number }; routines: { id: string; name: string }[] } | null>(null)
  /** The canvas search: nodes that don't match are dimmed and list rows are filtered. */
  const [query, setQuery] = useState('')
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

  /** The newest step a canvas node left in a run, and the routine it belongs to. */
  const stepOfNode = (key: string): { card: FlowRoutineCard; step: FlowStep } | null => {
    const [kind, id] = key.split(':')
    if (!id) return null
    const target = kind === 'media' ? mediaKey(id) : botKey(id)
    for (const card of canvas.routines) {
      const step = card.steps.find((candidate) => candidate.botId === target)
      if (step) return { card, step }
    }
    return null
  }

  /** The routine a canvas node belongs to, from its saved step or the links that name it. */
  const cardForNode = (key: string) => {
    const [kind, id] = key.split(':')
    if (!id) return undefined
    const target = kind === 'media' ? `media:${id}` : id
    return stepOfNode(key)?.card ?? canvas.routines.find((candidate) =>
      canvas.links.some((link) => link.routineId === candidate.routine.id
        && (link.fromBotId === target || link.toBotId === target)))
  }

  /** A step's name lives on the link or media node the host owns; an empty name clears it. */
  const renameStep = (key: string, name: string) => {
    const card = cardForNode(key)
    if (!card) {
      setNote(tr('That step is not part of a routine on this canvas.'))
      return
    }
    setNote(null)
    void flow.renameStep(card.routine.id, key, name)
  }

  /** Starts a new run at this step; the steps before it hand on their stored answers. */
  const runFromStep = async (key: string) => {
    if (offline) return
    const [kind, id] = key.split(':')
    if (!id) return
    const nodeKey = kind === 'media' ? mediaKey(id) : botKey(id)
    const card = stepOfNode(key)?.card
      ?? canvas.routines.find((candidate) =>
        canvas.links.some((link) => link.routineId === candidate.routine.id && (link.fromBotId === id || link.toBotId === id)))
    if (!card) {
      setNote(tr('That step is not part of a routine on this canvas.'))
      return
    }
    setNote(null)
    try {
      await (await store.ready()).call('runRoutineFlowFromStep', { botId, routineId: card.routine.id, nodeKey })
      setSelection({ kind: 'node', key })
      await flow.load()
    } catch (error) {
      setNote(`${tr('Could not run from this step.')} ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  /** Restores a step's saved settings so the same step can be run again with them. */
  const restoreSettings = async (target: { card: FlowRoutineCard; step: FlowStep }) => {
    const { card, step } = target
    const settings = settingsOf(canvas.stepSettings, step.id)
    if (!settings) return
    const mediaId = mediaEnd(step.botId)
    if (mediaId !== null) {
      const media = canvas.mediaNodes.find((node) => node.id === mediaId)
      if (!media) {
        setNote(tr('That picture step is gone.'))
        return
      }
      await flow.updateMedia(media.id, {
        model: settings.model,
        prompt: settings.prompt ?? '',
        aspect: settings.aspect ?? '1:1',
        duration: settings.duration ?? 6,
        resolution: settings.resolution ?? '480p',
        transparent: settings.transparent ?? false,
        voice: settings.voice ?? null,
      })
    } else {
      const link = canvas.links.find((item) => item.routineId === card.routine.id && item.toBotId === step.botId)
      if (!link) {
        setNote(tr('The link to this step is gone; add it again first.'))
        return
      }
      await flow.updateLink(link.id, settings.instruction ?? link.instruction, settings.model ?? null, settings.modelOptions ?? {})
    }
    setShownSettings(null)
    setSelection({ kind: 'node', key: mediaId !== null ? mediaKey(mediaId) : botKey(step.botId) })
  }

  /** Duplicates a picture step with its settings and the connections that fed it. The copy is selected and raised. */
  const duplicateMedia = async (media: FlowMedia) => {
    const places = new Map(layoutNodes(canvas).map((node) => [node.key, node]))
    const copy = await flow.addMedia(media.routineId, media.kind)
    if (!copy) return
    await flow.updateMedia(copy.id, {
      model: media.model,
      prompt: media.prompt,
      aspect: media.aspect,
      duration: media.duration,
      resolution: media.resolution,
      transparent: media.transparent,
      voice: media.voice,
    })
    for (const link of canvas.links.filter((item) => item.routineId === media.routineId && item.toBotId === mediaKey(media.id))) {
      await flow.connect(link.routineId, link.fromBotId, mediaKey(copy.id), link.instruction)
    }
    const at = places.get(mediaKey(media.id))
    if (at) await flow.move(mediaKey(copy.id), at.x + 40, at.y + 40)
    setSelection({ kind: 'node', key: mediaKey(copy.id) })
  }

  /** The menu a step's button opens: run from it, see its settings, duplicate a picture, or start at the first step. */
  const menuItems = (key: string): MenuItem[] => {
    const found = stepOfNode(key)
    const rows: MenuItem[] = []
    if (found) {
      rows.push({ title: 'Run from this step', icon: 'play.fill', action: () => void runFromStep(key) })
      rows.push({ title: 'Settings', icon: 'gearshape', action: () => setShownSettings(found) })
      // A deterministic retry start: the leftmost node of this routine, or the topmost in one column.
      const members = new Set<string>([found.card.routine.botId])
      for (const link of canvas.links) {
        if (link.routineId === found.card.routine.id) {
          members.add(link.fromBotId)
          members.add(link.toBotId)
        }
      }
      const first = leftmostNode(layoutNodes(canvas).filter((node) => node.kind !== 'routine' && members.has(node.id)))
      if (first && first.key !== key) {
        rows.push({ title: 'Run from the first step', icon: 'arrow.uturn.backward', action: () => void runFromStep(first.key) })
      }
    }
    const mediaId = key.startsWith('media:') ? key.slice(6) : null
    const media = mediaId ? canvas.mediaNodes.find((node) => node.id === mediaId) : null
    if (media) rows.push({ title: 'Duplicate', icon: 'doc.on.doc', action: () => void duplicateMedia(media) })
    return rows
  }

  return (
    <VStack gap={8} className="routine-flow">
      <HStack gap={8} wrap>
        <SegmentedChoice selection={mode} options={[{ id: 'canvas', label: 'Canvas' }, { id: 'list', label: 'List' }]} onChange={setMode} />
        <input
          className="routine-input flow-search"
          aria-label={tr('Search the canvas')}
          placeholder={tr('Search the canvas')}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
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
      {canvas.routines.length === 0 && mode === 'canvas' ? (
        <FlowEmptyState note={note} disabled={offline} onNew={() => setCreating(true)} />
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
                menuItems={menuItems}
                onRenameStep={renameStep}
                query={query}
                composer={{
                  busy: model.busy,
                  offline,
                  targets,
                  onSaveMedia: saveMedia,
                  onRunFromStep: (key) => void runFromStep(key),
                }}
                onRun={(id) => {
                  setSelection({ kind: 'node', key: `routine:${id}` })
                  void flow.test(id)
                }}
                onStop={() => void store.stop(botId)}
              />
            ) : (
              <FlowList canvas={canvas} bots={bots} selection={selection} onSelect={setSelection} query={query} note={note} disabled={offline} onNew={() => setCreating(true)} />
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
            onShowSettings={(card, step) => setShownSettings({ card, step })}
            onDuplicateMedia={(media) => void duplicateMedia(media)}
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
      <Sheet open={shownSettings !== null} onClose={() => setShownSettings(null)} width={520} height={620}>
        {shownSettings ? (
          <VStack gap={14} className="routine-flow-sheet flow-settings-sheet">
            <ModalHeader icon="gearshape" title={tr('Step settings')} onClose={() => setShownSettings(null)} />
            <VStack gap={8}>
              <Text weight="semibold">
                {stepNameOf(canvas, shownSettings.step.botId.startsWith('media:') ? shownSettings.step.botId : `bot:${shownSettings.step.botId}`)
                  || shownSettings.step.botId}
              </Text>
              <SavedSettings settings={settingsOf(canvas.stepSettings, shownSettings.step.id)} />
            </VStack>
            <HStack gap={8}>
              <Spacer />
              <Button
                disabled={offline || !settingsOf(canvas.stepSettings, shownSettings.step.id)}
                onClick={() => void restoreSettings(shownSettings)}
              >
                {tr('Use these settings')}
              </Button>
            </HStack>
          </VStack>
        ) : null}
      </Sheet>
    </VStack>
  )
}
