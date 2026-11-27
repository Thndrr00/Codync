import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { MediaKind } from '@shared/media'
import { draftOf, isGroup, type Bot } from '@shared/models'
import { Button, ChoicePicker, DropdownMenu, IconButton, SegmentedChoice, Spinner } from '../../../components/Controls'
import { Dialog, DialogField, ModalHeader, Sheet } from '../../../components/Overlay'
import { AnchoredMenu, type MenuItem } from '../../../components/Menu'
import { HStack, Spacer, VStack } from '../../../components/Stack'
import { Text } from '../../../components/Text'
import { useModel } from '../../../lib/observable'
import { prefs, usePref } from '../../../lib/prefs'
import {
  GRAPH_PROBLEM_TEXT, NODE_HEIGHT, NODE_WIDTH, botKey, canvasBotIds, canvasGraph, canvasRunning,
  canvas_media_kind, connectProblem, fieldsForStep, graphEdgeProblem, isFlowPaused, isFlowReadOnly, layoutNodes, mediaBatch,
  mediaEnd, mediaKey, mediaTitle, nextRunCursor, problemsForStep, specKeyOf, specText, stepNameOf,
  tidyLayout, unknownSettings, unsupported_media, visible_spot, workflowFileBytes,
  type FlowCanvas, type FlowField, type FlowLink, type FlowMedia, type FlowPosition, type FlowRoutineCard,
  type FlowStep, type SettingProblem, type View, type WorkflowFolderAction, type WorkflowFields, type WorkflowGitHubHistory,
  type WorkflowGraph, type WorkflowHistoryStep, type WorkflowNode, type WorkflowNodeSize, type WorkflowRunPage,
  type WorkflowStepSource, type WorkflowStroke, type WorkflowVersion,
} from '../../../lib/routine-flow.ts'
import { useTr } from '../../../lib/use-language'
import { useStore } from '../../../store/context'
import { RoutineFlowStore, type WorkflowImportBody, type WorkflowImportInfo } from '../../../store/routine-flow-store'
import { AllWorkflowsMenu } from './AllWorkflowsMenu'
import { FlowCanvasView, type ConnectOptions, type FlowSelection } from './FlowCanvas'
import {
  documentFromCanvas, type FlowChoice, type FlowConflict, type FlowDocument, type FlowSaveAnswer,
} from './flow-merge'
import {
  closeOtherWorkflows, closeWorkflow, closeWorkflowsRight, moveWorkflow, nextActiveTab, openWorkflow, restoreWorkflowTabs,
} from './flow-tabs'
import { groupsOf, groupSpot, newGroupId, outputType, parsePort, type CanvasWithGroups, type FlowGroup } from './flow-groups'
import { FlowInspector, type FlowTarget, type MediaSettings } from './FlowInspector'
import { FlowEmptyState } from './FlowEmptyState'
import { FlowList } from './FlowList'
import { FlowTabStrip } from './FlowTabStrip'
import { WorkflowImportDialog } from './WorkflowImportDialog'
import { clipped, hasMore, leftmostNode, settingsOf, type FlowStepSettings } from './flow-settings'
import { RunHistorySheet } from './RunHistorySheet'
import {
  decide, githubHistory, restoreVersion, retryStep, rotateWebhookSecret, runLastEvent, runs, saveGraph,
  stepSettings, stopRun, stopStep, versionGraph, versions, type WorkflowCall,
} from './workflow-host'
import { RoutineEditorView } from '../RoutineEditorView'
import './routine-flow.css'

type Mode = 'canvas' | 'list'

/** The bots a routine flow can hand work to: agents of their own, not hidden. */
const canTakeWork = (bot: Bot) => !bot.hidden && !isGroup(bot) && bot.backend !== 'external'

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
          {settings.batch ? <SettingsText label="Batch" value={String(mediaBatch(settings.batch))} /> : null}
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
  /** The open workflows, in tab order, on this device (ids only). The active one is session state. */
  const [openTabs, setOpenTabs] = usePref(prefs.openWorkflowTabs)
  const [activeId, setActiveId] = useState(botId)
  /** The host files workflows by routine id; a tab is the bot whose canvas holds its routines. */
  const [workflowList, setWorkflowList] = useState<{ id: string; name: string; botId: string }[]>([])
  const [mode, setMode] = useState<Mode>('canvas')
  const [selection, setSelection] = useState<FlowSelection>(null)
  const [note, setNote] = useState<string | null>(null)
  /** Files dropped or pasted on the canvas are still being turned into steps. */
  const [uploadingFiles, setUploadingFiles] = useState(false)
  /** Someone else's edit was folded into ours without a choice being needed. */
  const [mergeNote, setMergeNote] = useState(false)
  /** A save found fields both people changed: the merged document and the choices still needed. */
  const [conflict, setConflict] = useState<{ document: FlowDocument; conflicts: FlowConflict[] } | null>(null)
  const [choices, setChoices] = useState<Record<string, FlowChoice>>({})
  /** The step whose saved settings are open in the sheet. */
  const [shownSettings, setShownSettings] = useState<{ card: FlowRoutineCard; step: FlowStep } | null>(null)
  /** A workflow being renamed from the All workflows menu (the tabs rename inline). */
  const [renameDraft, setRenameDraft] = useState<{ id: string; name: string } | null>(null)
  const [creating, setCreating] = useState(false)
  const [choose, setChoose] = useState<{ from: string; to: string; at: { x: number; y: number }; routines: { id: string; name: string }[] } | null>(null)
  /** The canvas search: nodes that don't match are dimmed and list rows are filtered. */
  const [query, setQuery] = useState('')
  /** Keys being grouped, waiting for a name (Ctrl/Cmd+G). */
  const [groupDraft, setGroupDraft] = useState<string[] | null>(null)
  const [groupName, setGroupName] = useState('')
  /** The drawing a tidy replaced, so Undo can put the steps back. */
  const [tidyUndo, setTidyUndo] = useState<{ positions: FlowPosition[]; groups: FlowGroup[] } | null>(null)
  /** A file being imported: its bytes and the names to map before the host makes the paused copy. */
  const [importDraft, setImportDraft] = useState<{ info: WorkflowImportInfo; content: string; suggestedName?: string } | null>(null)
  /** Steps that hold a secret, which the host refuses to export until they are changed. */
  const [exportBlocked, setExportBlocked] = useState<string[] | null>(null)
  // The portable graph is additive: an older host sends none of it and the routine canvas works on.
  const [workflowVersions, setWorkflowVersions] = useState<WorkflowVersion[]>([])
  const [versionPreview, setVersionPreview] = useState<{ version: WorkflowVersion; graph: WorkflowGraph } | null>(null)
  const [showRuns, setShowRuns] = useState(false)
  const [runPage, setRunPage] = useState<WorkflowRunPage | null>(null)
  const [runsError, setRunsError] = useState<string | null>(null)
  const [sourcesByStep, setSourcesByStep] = useState<Record<string, WorkflowStepSource[] | null>>({})
  const [webhookSecret, setWebhookSecret] = useState<string | null>(null)
  const [deliveries, setDeliveries] = useState<WorkflowGitHubHistory | null>(null)
  const [detailSettings, setDetailSettings] = useState<{ stepId: string; value: unknown } | null>(null)
  const offline = store.isOffline
  const rev = store.flowRev(activeId)
  /** Where a node added by hand lands: the last pan/zoom the canvas reported, not a saved position. */
  const visibleSpot = useRef(visible_spot({ x: 0, y: 0, scale: 1 }, { width: 800, height: 500 }))
  const rememberView = useCallback((view: View, box: { width: number; height: number }) => {
    visibleSpot.current = visible_spot(view, box)
  }, [])

  useEffect(() => {
    flow.resume()
    void flow.load()
    return () => flow.dispose()
  }, [flow])
  // A parent opening another workflow adds its tab and selects it; a launch restores the saved strip.
  useEffect(() => {
    setOpenTabs(restoreWorkflowTabs(openTabs, botId))
    setActiveId(botId)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [botId])
  // The tab strip owns which workflow's canvas the store shows from here on.
  useEffect(() => { flow.switchTo(activeId) }, [flow, activeId])
  useEffect(() => { void flow.loadFolders() }, [flow])
  const botIdsKey = [...store.bots.keys()].join('\u0000')
  useEffect(() => {
    let live = true
    const ids = botIdsKey ? botIdsKey.split('\u0000') : []
    void flow.listWorkflows(ids).then((list) => {
      if (live) setWorkflowList(list)
    }).catch((error: unknown) => {
      if (live) setNote(error instanceof Error ? error.message : String(error))
    })
    return () => { live = false }
  }, [flow, botIdsKey])
  // The host says the canvas changed (another client, a step finishing): load it again.
  useEffect(() => {
    if (rev > 0) void flow.load()
  }, [rev, flow])

  const canvas = model.canvas as CanvasWithGroups | null
  const bots = store.bots
  const roster = useMemo(() => [...bots.values()].filter(canTakeWork), [bots])

  // --- The title bar's workflow tabs and the "All workflows" tree --------------------------------

  /** Tabs are bot canvases; the host files workflows by routine id, so the menu lists routines. */
  const workflows = useMemo(() => workflowList.map((workflow) => ({
    ...workflow,
    name: workflow.name?.trim() || bots.get(workflow.botId)?.name?.trim() || tr('Untitled workflow'),
  })), [workflowList, bots, tr])
  const currentWorkflowId = workflows.find((workflow) => workflow.botId === activeId)?.id
    ?? canvas?.routines.find((card) => card.routine.botId === activeId)?.routine.id
    ?? canvas?.routines[0]?.routine.id ?? null
  /** Pause skips triggers; a manual run still works. A newer setting makes the whole workflow read-only. */
  const paused = canvas ? isFlowPaused(canvas, currentWorkflowId ?? '') : false
  const readOnly = isFlowReadOnly(canvas)
  const nameOfWorkflow = (id: string) => bots.get(id)?.name?.trim() || tr('Untitled workflow')
  const isRunning = (id: string) => {
    const status = bots.get(id)?.status
    return status === 'working' || status === 'needsInput' || (id === activeId && canvasRunning(canvas))
  }
  const openTab = (id: string) => {
    setOpenTabs(openWorkflow(openTabs, id))
    setActiveId(id)
    setSelection(null)
  }
  const closeTab = (id: string) => {
    const next = closeWorkflow(openTabs, id)
    setOpenTabs(next)
    const follow = nextActiveTab(openTabs, id, activeId)
    if (follow) setActiveId(follow)
  }
  const closeOthers = (id: string) => {
    setOpenTabs(closeOtherWorkflows(openTabs, id))
    setActiveId(id)
  }
  const closeRight = (id: string) => setOpenTabs(closeWorkflowsRight(openTabs, id))
  const reorderTab = (from: string, to: string) => setOpenTabs(moveWorkflow(openTabs, from, to))
  /** A tab's name is its workflow's name, which on Codync is the bot's name. */
  const renameWorkflow = async (id: string, name: string) => {
    const bot = bots.get(id)
    if (!bot) return
    try {
      await store.save({ ...draftOf(bot), name })
    } catch (error) {
      setNote(`${tr('Could not rename the workflow.')} ${error instanceof Error ? error.message : String(error)}`)
    }
  }
  const applyFolderAction = (action: WorkflowFolderAction) => { void flow.folderAction(action) }

  // --- Portable workflow graph: every change goes through the host with the revision we read -----

  const graph = canvasGraph(canvas)
  const workflowCall: WorkflowCall = (method, body) => store.ready().then((client) => client.call(method, body))
  const stepsByNode: Record<string, WorkflowHistoryStep | null> = canvas?.workflowSteps ?? {}
  const graphChange = (change: (current: WorkflowGraph) => WorkflowGraph) => {
    if (!canvas || !currentWorkflowId || offline || readOnly) return
    const current = graph ?? { nodes: [], edges: [], strokes: [] }
    void saveGraph(workflowCall, currentWorkflowId, canvas.rev, change(current)).then((answer) => {
      if (answer?.ok === false) setNote(tr('This workflow changed on the host. Reload it before drawing again.'))
      else void flow.load()
    }).catch((error: unknown) => {
      setNote(`${tr('Could not save the workflow.')} ${error instanceof Error ? error.message : String(error)}`)
    })
  }
  const saveNode = (node: WorkflowNode, fields: WorkflowFields, size?: WorkflowNodeSize) => {
    graphChange((current) => ({
      ...current,
      nodes: current.nodes.map((item) => item.id === node.id
        ? { ...item, fields, size: size ?? item.size }
        : item),
    }))
  }
  const decideStep = (nodeId: string, approved: boolean) => {
    const step = stepsByNode[nodeId]
    if (!step) return
    void decide(workflowCall, step.id, approved).then((ok) => {
      if (ok) void flow.load()
      else setNote(tr('This host cannot answer an approval step yet.'))
    }).catch(() => setNote(tr('Could not save the decision.')))
  }
  const loadVersions = () => {
    if (!currentWorkflowId || offline) return
    void versions(workflowCall, currentWorkflowId).then((list) => { if (list) setWorkflowVersions(list) }).catch(() => {})
  }
  const showVersion = (version: WorkflowVersion) => {
    if (!currentWorkflowId) return
    void versionGraph(workflowCall, currentWorkflowId, version.id).then((preview) => {
      if (preview) setVersionPreview({ version, graph: preview })
    }).catch(() => setNote(tr('Could not read that version.')))
  }
  const restore = (version: WorkflowVersion) => {
    if (!currentWorkflowId || offline) return
    void restoreVersion(workflowCall, currentWorkflowId, version.id).then((answer) => {
      if (answer) {
        setVersionPreview(null)
        setNote(tr('Version restored.'))
        void flow.load()
      }
    }).catch(() => setNote(tr('Could not restore that version.')))
  }
  const loadRuns = (before: string | null) => {
    if (!currentWorkflowId || offline) return
    void runs(workflowCall, currentWorkflowId, before, 20).then((page) => {
      if (page) {
        setRunPage((previous) => before ? { ...page, runs: [...(previous?.runs ?? []), ...page.runs] } : page)
        setRunsError(null)
        void Promise.all(page.runs.flatMap((detail) => detail.steps.map(async (step) => {
          const settings = await stepSettings(workflowCall, step.id).catch(() => null)
          return [step.id, settings?.sources ?? null] as const
        }))).then((entries) => setSourcesByStep(Object.fromEntries(entries) as Record<string, WorkflowStepSource[] | null>))
      }
    }).catch(() => setRunsError(tr('Could not read this page of runs.')))
  }
  const loadDeliveries = (nodeId: string) => {
    void githubHistory(workflowCall, nodeId).then((history) => { if (history) setDeliveries(history) }).catch(() => {})
  }
  const rotateSecret = (nodeId: string) => {
    if (!currentWorkflowId) return
    void rotateWebhookSecret(workflowCall, currentWorkflowId, nodeId).then((secret) => {
      if (secret) setWebhookSecret(secret)
      else setNote(tr('This host cannot rotate webhook secrets yet.'))
    }).catch(() => setNote(tr('Could not rotate the secret.')))
  }
  const replayEvent = (nodeId: string) => {
    if (!currentWorkflowId) return
    void runLastEvent(workflowCall, currentWorkflowId, nodeId, crypto.randomUUID()).then((detail) => {
      if (detail) {
        setNote(tr('Started a run with the last event.'))
        void flow.load()
      } else setNote(tr('This host cannot run the last event yet.'))
    }).catch(() => setNote(tr('Could not run the last event.')))
  }

  // --- Step fields, imported/exported workflows and pause ---------------------------------------

  const fieldsFor = (nodeKey: string): FlowField[] => canvas ? fieldsForStep(canvas, nodeKey) : []
  const problemsFor = (nodeKey: string) => canvas ? problemsForStep(canvas, nodeKey) : {}
  const answerFor = (nodeKey: string) => {
    const spec = canvas?.answerSpecs?.[specKeyOf(nodeKey)]
    return specText(typeof spec === 'object' ? spec : undefined, 'answer') || (typeof spec === 'string' ? spec : '')
  }
  const checkFor = (nodeKey: string) => {
    const spec = canvas?.answerSpecs?.[specKeyOf(nodeKey)]
    return specText(typeof spec === 'object' ? spec : undefined, 'check') || (canvas?.checkSpecs?.[nodeKey] ?? '')
  }
  const saveAnswer = (nodeKey: string, answer: string) => {
    if (!canvas || readOnly || !currentWorkflowId) return
    void flow.setAnswers(currentWorkflowId, nodeKey, answer, checkFor(nodeKey))
  }
  const saveCheck = (nodeKey: string, check: string) => {
    if (!canvas || readOnly || !currentWorkflowId) return
    void flow.setAnswers(currentWorkflowId, nodeKey, answerFor(nodeKey), check)
  }

  /** Refuses a save the host would reject and tells the person which values it does know. */
  const refuseSetting = (problems: SettingProblem[]): boolean => {
    if (problems.length === 0) return false
    const first = problems[0]
    setNote(first.newer
      ? tr('This step uses a setting from a newer version of Codync. The workflow is read-only.')
      : tr('Unknown setting {name}. Use {values}.').replace('{name}', first.name).replace('{values}', first.values.join(', ')))
    return true
  }

  const workflowName = currentWorkflowId
    ? workflows.find((workflow) => workflow.id === currentWorkflowId)?.name ?? tr('Untitled workflow')
    : tr('Untitled workflow')
  const exportWorkflow = async () => {
    if (!currentWorkflowId || offline) return
    setNote(null)
    const file = await flow.exportFlow(currentWorkflowId)
    if (!file) return
    if ('secrets' in file) {
      setExportBlocked(file.secrets)
      return
    }
    // The file keeps steps, notes and files; secrets never leave the computer.
    const url = URL.createObjectURL(new Blob([workflowFileBytes(file.content)], { type: 'application/json' }))
    const anchor = document.createElement('a')
    anchor.href = url
    anchor.download = file.name.endsWith('.workflow') ? file.name : `${file.name || workflowName}.openbot-workflow`
    anchor.click()
    URL.revokeObjectURL(url)
  }
  const inspectFile = async (content: string, suggestedName?: string) => {
    setNote(null)
    const info = await flow.inspectImport(content)
    if (info) setImportDraft({ info, content, suggestedName })
  }
  const duplicateWorkflow = async () => {
    if (!currentWorkflowId || offline) return
    const file = await flow.exportFlow(currentWorkflowId)
    if (!file) return
    if ('secrets' in file) {
      setExportBlocked(file.secrets)
      return
    }
    // Importing with the same agent/channel names makes the paused copy the host returns.
    await inspectFile(file.content, tr('{name} copy').replace('{name}', workflowName))
  }
  const pickImportFile = async () => {
    if (offline) return
    try {
      const picked = await window.codync.app.pickFiles()
      const first = picked[0]
      if (first) await inspectFile(new TextDecoder().decode(first.data))
    } catch {
      setNote(tr('Could not read that workflow file.'))
    }
  }
  const commitImport = async (body: WorkflowImportBody) => {
    const ok = await flow.importFlow(body)
    setImportDraft(null)
    if (ok) {
      setNote(null)
      void flow.loadFolders()
    }
  }

  const connectable = (c: FlowCanvas, from: string, to: string) =>
    c.routines.filter((r) => connectProblem(c, r.routine.id, from, to) === null)

  const connect = async (routineId: string, from: string, to: string) => {
    setNote(null)
    const link = await flow.connect(routineId, from, to, '')
    if (link) setSelection({ kind: 'edge', id: link.id })
  }

  const dropped = (from: string, to: string, at: { x: number; y: number }, options?: ConnectOptions) => {
    if (!canvas || offline) return
    // A portable edge carries a typed port and, from a condition, a yes or no branch. Both the
    // client's pre-check and the host's graph validation must agree before a line is saved.
    const source = graph?.nodes.find((node) => node.id === from) ?? null
    const target = graph?.nodes.find((node) => node.id === to) ?? null
    if (source || target || options?.branch) {
      const type = options?.type ?? (source?.kind === 'asset' ? 'text' : 'text')
      const problem = graphEdgeProblem(source?.kind ?? 'llm', target?.kind ?? 'llm', type, options?.branch)
      if (problem) {
        setNote(tr(GRAPH_PROBLEM_TEXT[problem] ?? 'These steps cannot be connected.'))
        return
      }
      graphChange((current) => ({
        ...current,
        edges: [...current.edges, { id: crypto.randomUUID(), from, to, type, ...(options?.branch ? { branch: options.branch } : {}) }],
      }))
      return
    }
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

  // The strip stays visible while a workflow opens: only the canvas area shows the spinner.
  const titleBar = (
    <HStack gap={8} align="center" className="flow-title-bar">
      <AllWorkflowsMenu
        workflows={workflows}
        current={currentWorkflowId ?? ''}
        folders={flow.folders}
        loading={flow.foldersLoading}
        error={flow.foldersError}
        onOpen={(workflow) => {
          openTab(workflow.botId)
          setSelection({ kind: 'node', key: `routine:${workflow.id}` })
        }}
        onNew={() => setCreating(true)}
        onRename={(workflow) => setRenameDraft({ id: workflow.botId, name: nameOfWorkflow(workflow.botId) })}
        onFolderAction={applyFolderAction}
        paused={paused}
        onPause={() => { if (currentWorkflowId) { setNote(null); void flow.setPaused(currentWorkflowId, !paused) } }}
        versions={workflowVersions}
        onVersions={loadVersions}
        onVersionGraph={showVersion}
        onRestore={restore}
        onShowRuns={() => { setShowRuns(true); loadRuns(null) }}
        onDuplicate={() => void duplicateWorkflow()}
        onExport={() => void exportWorkflow()}
        onImport={() => void pickImportFile()}
      />
      <FlowTabStrip
        tabs={openTabs}
        active={activeId}
        nameOf={nameOfWorkflow}
        running={isRunning}
        onSelect={openTab}
        onClose={closeTab}
        onCloseOthers={closeOthers}
        onCloseRight={closeRight}
        onReorder={reorderTab}
        onRename={(id, name) => void renameWorkflow(id, name)}
      />
      <IconButton title={tr('New workflow')} icon="plus" disabled={offline} onClick={() => setCreating(true)} />
    </HStack>
  )

  if (flow.unsupported) {
    return <Text size="callout" tone="secondary" className="outlined-card flow-state">{tr('This computer\'s host doesn\'t support routine flows yet. Update Codync on it.')}</Text>
  }
  if (!canvas) {
    return (
      <VStack gap={8} className="routine-flow">
        {titleBar}
        {model.error ? (
          <VStack gap={8} className="outlined-card flow-state">
            <Text size="callout" tone="danger">{model.error}</Text>
            <Button kind="secondary" onClick={() => void flow.load()}>{tr('Try again')}</Button>
          </VStack>
        ) : (
          <HStack gap={8} className="outlined-card flow-state"><Spinner /><Text size="callout" tone="secondary">{tr('Loading the flow…')}</Text></HStack>
        )}
      </VStack>
    )
  }

  const onCanvas = new Set(canvasBotIds(canvas))
  const addable = roster.filter((bot) => !onCanvas.has(bot.id))
  const addBot = (bot: Bot) => {
    const spot = visibleSpot.current
    void flow.place(bot.id, spot.x, spot.y)
  }
  const saveLink = (link: FlowLink, text: string, model: string | null, modelOptions: Record<string, string>) => {
    if (refuseSetting(unknownSettings('bot', modelOptions, canvas.nodeRegistry))) return
    void flow.updateLink(link.id, text, model, modelOptions)
  }
  const disconnect = async (link: FlowLink) => {
    setSelection(null)
    await flow.disconnect(link.id)
  }
  const addMedia = async (routineId: string, kind: MediaKind) => {
    setNote(null)
    const media = await flow.addMedia(routineId, kind)
    if (media) setSelection({ kind: 'node', key: mediaKey(media.id) })
  }
  const saveMedia = (media: FlowMedia, settings: MediaSettings) => {
    const kind = media.kind === 'video' ? 'video' : 'image'
    if (refuseSetting(unknownSettings(kind, settings as unknown as Record<string, unknown>, canvas.nodeRegistry))) return
    void flow.updateMedia(media.id, settings)
  }
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
          botId: activeId, field: item.field, key: item.key, choice: choices[conflictId(item)] ?? 'local',
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

  // Groups and step positions are drawing-only data on the host's canvas. The save carries the
  // whole flow document with the revision we started from, so the host can merge someone else's
  // edits field by field and writes the merged drawing back in the same transaction.
  const saveDocument = async (document: FlowDocument): Promise<boolean> => {
    if (!canvas || offline) return false
    setMergeNote(false)
    try {
      const answer = await (await store.ready()).call<FlowSaveAnswer>('saveRoutineFlow', { botId: activeId, baseRev: canvas.rev, document })
      if (answer.conflicts?.length) {
        setConflict({ document: answer.document, conflicts: answer.conflicts })
        setChoices({})
        return false
      }
      if (answer.merged) setMergeNote(true)
    } catch {
      setNote(tr('Could not save the group.'))
      return false
    }
    await flow.load()
    return true
  }
  const saveGroups = async (next: FlowGroup[]) => {
    if (!canvas) return
    await saveDocument(documentFromCanvas(canvas, next))
  }

  /** One tidy (and its Undo) saves the whole drawing at once, so the steps never land half moved. */
  const tidy = async () => {
    if (!canvas || offline) return
    const before = { positions: canvas.positions, groups: groupsOf(canvas) }
    const layout = tidyLayout(canvas, before.groups)
    const groups = before.groups.map((group) => {
      const at = layout.groups.find((place) => place.id === group.id)
      return at ? { ...group, x: at.x, y: at.y } : group
    })
    if (await saveDocument(documentFromCanvas({ ...canvas, positions: layout.positions }, groups))) setTidyUndo(before)
  }
  const undoTidy = async () => {
    if (!canvas || !tidyUndo || offline) return
    if (await saveDocument(documentFromCanvas({ ...canvas, positions: tidyUndo.positions }, tidyUndo.groups))) setTidyUndo(null)
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
  /** A paste or drop of two or more files makes one media step per file, all in one group in the middle of the view. */
  const addFiles = async (files: { name: string }[], at: { x: number; y: number }) => {
    if (!canvas || offline) return
    const routine = canvas.routines.find((r) => r.routine.botId === activeId) ?? canvas.routines[0]
    if (!routine) {
      setNote(tr('Add a routine before dropping files.'))
      return
    }
    const members: string[] = []
    setUploadingFiles(true)
    try {
      for (const file of files) {
        const kind = canvas_media_kind(file.name)
        if (!kind) {
          setNote(tr(unsupported_media(file.name) ?? ''))
          continue
        }
        // The host's media steps are pictures or clips; a sound or a clip takes the clip step and
        // keeps its own name, so the composer can show which file it stands for.
        const media = await flow.addMedia(routine.routine.id, kind === 'image' ? 'image' : 'video')
        if (!media) return
        await flow.updateMedia(media.id, { prompt: file.name })
        members.push(mediaKey(media.id))
      }
      if (members.length < 2) return
      // The paste point is the middle of the visible canvas, not a saved position from another
      // workflow that might be far off screen (OpenBot 0.70 workflow paste fix).
      const spot = { x: Math.round(at.x - NODE_WIDTH / 2), y: Math.round(at.y - NODE_HEIGHT / 2) }
      await saveGroups([...groupsOf(canvas), {
        id: newGroupId(groupsOf(canvas)), name: tr('New group'), members, collapsed: false, x: spot.x, y: spot.y,
      }])
    } finally {
      setUploadingFiles(false)
    }
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
      await (await store.ready()).call('runRoutineFlowFromStep', { botId: activeId, routineId: card.routine.id, nodeKey })
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
        batch: mediaBatch(settings.batch ?? media.batch),
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
      batch: mediaBatch(media.batch),
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
      {titleBar}
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
      {readOnly ? <Text size="caption" tone="attention">{tr('This workflow uses a setting from a newer version of Codync. Update Codync to change it; a manual run still works.')}</Text> : null}
      {model.error ? <Text size="caption" tone="danger">{model.error}</Text> : null}
      {note ? <Text size="caption" tone="danger">{note}</Text> : null}
      {uploadingFiles ? (
        <HStack gap={6} align="center"><Spinner size={12} /><Text size="caption" tone="secondary">{tr('Adding files to the canvas…')}</Text></HStack>
      ) : null}
      {mergeNote ? <Text size="caption" tone="attention" className="flow-merge-note">{tr('Someone else edited this flow; their changes were merged in.')}</Text> : null}
      <HStack gap={8} align="stretch" className="flow-body">
        <div className="flow-main">
          {mode === 'canvas' ? (
            <div className="flow-canvas-wrap">
              <FlowCanvasView
                canvas={canvas}
                bots={bots}
                selection={selection}
                disabled={offline || readOnly || model.busy}
                onSelect={setSelection}
                onMove={(key, x, y) => void flow.move(key, x, y)}
                onMoveGroup={(id, x, y) => void moveGroup(id, x, y)}
                onTidy={() => void tidy()}
                onUndoTidy={tidyUndo ? () => void undoTidy() : undefined}
                onConnect={dropped}
                onGroup={(keys) => { setGroupDraft(keys); setGroupName('') }}
                onUngroup={(id) => void ungroup(id)}
                onToggleGroup={(id) => void toggleGroup(id)}
                onFiles={(files, at) => void addFiles(files, at)}
                onDelete={(key) => void flow.removeNode(key)}
                menuItems={menuItems}
                onRenameStep={renameStep}
                query={query}
                onViewChange={rememberView}
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
                onStop={() => void store.stop(activeId)}
                workflow={graph || canvas.workflowTriggers || canvas.needsNewerApp ? {
                  graph,
                  triggers: canvas.workflowTriggers ?? [],
                  needsNewerApp: canvas.needsNewerApp === true,
                  steps: stepsByNode,
                  onDecide: decideStep,
                  onResize: (nodeId, size) => {
                    const node = graph?.nodes.find((item) => item.id === nodeId)
                    if (node) saveNode(node, node.fields, size)
                  },
                  onStroke: (stroke: WorkflowStroke) => graphChange((current) => ({ ...current, strokes: [...current.strokes, stroke] })),
                } : undefined}
              />
              {canvas.routines.length === 0 ? (
                <div className="flow-empty-overlay">
                  <FlowEmptyState note={note} disabled={offline} onNew={() => setCreating(true)} />
                </div>
              ) : null}
              {paused ? (
                <div className="flow-paused-badge">
                  <Text weight="semibold">{tr('Paused')}</Text>
                  <Button
                    kind="secondary"
                    disabled={offline}
                    onClick={() => { if (currentWorkflowId) void flow.setPaused(currentWorkflowId, false) }}
                  >
                    {tr('Resume')}
                  </Button>
                </div>
              ) : null}
              {readOnly ? <div className="flow-readonly-badge"><Text size="caption" tone="attention">{tr('Read-only')}</Text></div> : null}
              {canvas.needsNewerApp ? (
                <div className="flow-readonly-badge"><Text size="caption" tone="attention">{tr('Needs newer app')}</Text></div>
              ) : null}
            </div>
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
          readOnly={readOnly}
          fieldsFor={fieldsFor}
          problemsFor={problemsFor}
          answerFor={answerFor}
          checkFor={checkFor}
          onSaveAnswer={saveAnswer}
          onSaveCheck={saveCheck}
          targets={targets}
          onSaveInstruction={saveLink}
          onDisconnect={(link) => void disconnect(link)}
          onTest={(id) => void flow.test(id)}
          onRunFromStep={(nodeKey) => void runFromStep(nodeKey)}
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
          workflow={{ graph: graph ?? { nodes: [], edges: [], strokes: [] }, triggers: canvas.workflowTriggers ?? [],
            steps: stepsByNode, readOnly: readOnly || offline || model.busy, fieldsFor, problemsFor,
            answerFor, checkFor, onSaveAnswer: saveAnswer, onSaveCheck: saveCheck,
            onSaveFields: (node, fields) => saveNode(node, fields),
            onDecide: decideStep,
            onRotateSecret: rotateSecret,
            onRunLastEvent: replayEvent,
            onLoadTriggerHistory: loadDeliveries,
            triggerHistory: deliveries,
            secret: webhookSecret,
          }}
        />
      </HStack>
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
                botId={activeId}
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
        open={renameDraft !== null}
        title={tr('Rename workflow')}
        actions={[{
          title: tr('Rename'),
          disabled: !renameDraft?.name.trim(),
          action: () => {
            if (renameDraft?.name.trim()) void renameWorkflow(renameDraft.id, renameDraft.name.trim())
          },
        }]}
        onClose={() => setRenameDraft(null)}
      >
        {renameDraft ? (
          <DialogField
            value={renameDraft.name}
            onChange={(name) => setRenameDraft({ ...renameDraft, name })}
            onSubmit={() => {
              if (renameDraft.name.trim()) void renameWorkflow(renameDraft.id, renameDraft.name.trim())
              setRenameDraft(null)
            }}
            placeholder={tr('Workflow name')}
            label={tr('Workflow name')}
          />
        ) : null}
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
      <Dialog
        open={exportBlocked !== null}
        title={tr('Remove the secrets before you export.')}
        message={tr('Found in: {fields}').replace('{fields}', (exportBlocked ?? []).join(', '))}
        actions={[]}
        onClose={() => setExportBlocked(null)}
      />
      <Sheet open={importDraft !== null} onClose={() => setImportDraft(null)} width={560} height={620}>
        {importDraft ? (
          <WorkflowImportDialog
            info={importDraft.info}
            content={importDraft.content}
            suggestedName={importDraft.suggestedName}
            candidates={[...bots.values()].map((bot) => ({ id: bot.id, name: bot.name }))}
            onClose={() => setImportDraft(null)}
            onImport={(body) => void commitImport(body)}
          />
        ) : null}
      </Sheet>
      <RunHistorySheet
        open={showRuns}
        onClose={() => setShowRuns(false)}
        page={runPage}
        loading={model.busy}
        error={runsError}
        sourcesByStep={sourcesByStep}
        onLoadMore={() => loadRuns(nextRunCursor(runPage))}
        onRetry={(stepId) => {
          void retryStep(workflowCall, stepId, crypto.randomUUID()).then((answer) => {
            if (answer) void flow.load()
            else setNote(tr('This host cannot retry that step yet.'))
          }).catch(() => setNote(tr('Could not retry the step.')))
        }}
        onStopStep={(stepId) => {
          void stopStep(workflowCall, stepId).then((ok) => { if (ok) void flow.load() })
            .catch(() => setNote(tr('Could not stop the step.')))
        }}
        onStopRun={(runId) => {
          void stopRun(workflowCall, runId).then((ok) => { if (ok) void flow.load() })
            .catch(() => setNote(tr('Could not stop the run.')))
        }}
        onShowSettings={(stepId) => {
          void stepSettings(workflowCall, stepId).then((settings) => setDetailSettings({ stepId, value: settings }))
            .catch(() => setNote(tr('Could not read the settings of that step.')))
        }}
      />
      <Dialog
        open={versionPreview !== null}
        title={tr('Version graph')}
        message={versionPreview ? tr('{kind} · {at}').replace('{kind}', versionPreview.version.kind).replace('{at}', new Date(versionPreview.version.at).toLocaleString()) : ''}
        actions={versionPreview ? [{
          title: tr('Restore this version'),
          action: () => restore(versionPreview.version),
        }] : []}
        onClose={() => setVersionPreview(null)}
      >
        {versionPreview ? (
          <Text size="callout" className="flow-version-graph">
            {tr('{count} nodes, {edges} lines').replace('{count}', String(versionPreview.graph.nodes.length)).replace('{edges}', String(versionPreview.graph.edges.length))}
          </Text>
        ) : null}
      </Dialog>
      <Dialog
        open={detailSettings !== null}
        title={tr('Settings used')}
        message={detailSettings?.stepId ?? ''}
        actions={[]}
        onClose={() => setDetailSettings(null)}
      >
        {detailSettings ? (
          <Text size="callout" className="flow-version-graph">{JSON.stringify(detailSettings.value, null, 2)}</Text>
        ) : null}
      </Dialog>
    </VStack>
  )
}
