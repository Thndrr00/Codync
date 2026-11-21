import { VoiceReceipt } from './VoiceReceipt'
import { RepliesView } from './RepliesView'
import { ForwardSheet } from './ForwardSheet'
import { FileView, type OpenFile } from './FileView'
import { IslandBudget } from '../island/IslandBudget'
import { IslandTerminal } from '../island/IslandTerminal'
import { useIslandThreadNavigation } from './use-island-thread-navigation'
import { memo, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react'
import { draftOf, folderName, isGroup, isWorking, isWorkingIn, sharedWorkspacePeers, type Bot, type BotApp, type BotDraft, type Entry, type SharedFile } from '@shared/models'
import { BotAvatar, GroupAvatar } from '../../components/Avatar'
import { BotAppPanel } from '../../components/BotAppPanel'
import { Spinner } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { ModalHeader, Sheet, SheetPage, usePresence } from '../../components/Overlay'
import { HStack, IconSlot, Spacer, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { prefs, usePref } from '../../lib/prefs'
import { capTabs, chatKey, markWindowActive, restoreSavedTabs, safeArtifactTab, safeFileTab, sameTab, windowToken, type ChatRef } from '../../lib/open-tabs'
import { connectedThrough } from '../../lib/relay-copy'
import { useTr } from '../../lib/use-language'
import { useScreenViewed } from '../../store/analytics'
import { useStore } from '../../store/context'
import { UpdateNeededCard } from '../UpdateNeededCard'
import { RosterHistory } from '../RosterControls'
import { buildChat, rowIn, useArrivals, type ChatItem } from './chat-items'
import { newestHealthNotices } from './health-notices'
import { BotConversationView, BotMessageRow } from './BotConversation'
import { CallConversationView, CallLineRow } from '../call/CallRows'
import { hidingCalls } from '@shared/calls'
import { devServerSummary } from '@shared/dev-servers'
import { ChatRow, TimeSeparator } from './ChatRows'
import { DevServerTitle } from './DevServersSection'
import { WorkingTransition } from './WorkingTransition'
import { RoutineNotice } from './RoutineNotice'
import { Composer } from './Composer'
import { PresenceRow } from './PresenceRow'
import { QueuedMessages } from './QueuedMessages'
import { fill } from '../../lib/fill.ts'
import { outsideQueue } from './queued-messages'
import { ChatSidePanel } from './ChatSidePanel'
import { DetailsPanel, type DetailsTab } from './DetailsPanel'
import { useNewMessageCount } from './new-message-counter'
import { activityLine } from './activity-line'
import { TraceView } from './TraceView'
import { workingSteps } from './trace-turns'
import { useReading } from './reading'
import { GroupEditorView } from '../bots/GroupEditorView'
import { BotTemplateView } from '../bots/BotTemplateView'
import { BotEditorView } from '../bots/BotEditorView'
import { FormSheet } from '../marketplace/pieces'
import { CallView } from '../call/CallView'
import { MacChatButton } from './MacChatButton'
import { TabStrip } from '../../components/TabStrip'
import { ChromeIconButton } from '../../components/Chrome'
import { ArtifactView } from '../library/ArtifactView'
import { KofiInviteCard } from './KofiInvite'
import { useKofiInviteAt } from '../../lib/use-kofi-invite'
import './thread.css'
import './composer.css'
import './chat-controls.css'
import './media.css'
import { MessageContext } from '../MessageContext'
import { PinsSheet } from './PinsSheet'

const WIDE = 740

/**
 * One endless conversation with a bot or a group chat. Only deliberate messages show here; tool
 * calls and thinking live in the "Full conversation" sheet. Replying to a message quotes it in the composer.
 */
export const ThreadView = memo(function ThreadView({ botId }: { botId: string }) {
  const tr = useTr()
  const store = useStore()
  const bot = store.bots.get(botId) ?? null
  // Tabs saved for this chat on this device come back on launch only while the switch is on, and
  // only for this computer and window (OpenBot 0.75.0). The store snapshot is loaded before a chat
  // is shown, so ids that are already gone are simply dropped instead of restoring foreign data.
  const chatRef = useMemo<ChatRef>(() => ({ computerId: store.computer.id, botId }), [store.computer.id, botId])
  const windowId = useMemo(() => windowToken(), [])
  const restored = useMemo(() => restoreSavedTabs(prefs.openTabsByChat.get(), chatRef, windowId, prefs.restoreLastWorkspace.get()), [chatRef, windowId])
  const initialTabs = useMemo(() => {
    const files: OpenFile[] = []
    const artifacts: string[] = []
    for (const tab of restored?.tabs ?? []) {
      if (tab.kind === 'artifact') {
        artifacts.push(tab.artifactId)
        continue
      }
      const entry = store.entries.get(botId)?.find((item) => item.id === tab.entryId && item.botId === botId)
      const file = entry?.data.files?.find((item) => item.id === tab.fileId)
      if (entry && file) files.push({ entry, file, entryId: entry.id })
    }
    const active = restored?.active
    return {
      files,
      artifacts,
      activeFile: active?.kind === 'file'
        ? (files.some((item) => item.file.id === active.fileId) ? active.fileId : null)
        : (active ? null : files[0]?.file.id ?? null),
      activeArtifact: active?.kind === 'artifact' && artifacts.includes(active.artifactId) ? active.artifactId : null,
    }
  }, [restored, store, botId])
  const [showTrace, setShowTrace] = useState(false)
  /** "New chat" only hides the old messages from this view; the palette's history search still finds them. */
  const [showOlder, setShowOlder] = useState(false)
  /** The chat's pin list. */
  const [showPins, setShowPins] = useState(false)
  const [replyTo, setReplyTo] = useState<Entry | null>(null)
  const cancelReply = useCallback(() => setReplyTo(null), [])
  const [botChat, setBotChat] = useState<string | null>(null)
  /** The voice call whose conversation is open (its line's entry id). */
  const [callLine, setCallLine] = useState<string | null>(null)
  const [threadRoot, setThreadRoot] = useState<string | null>(null)
  /** The message the forward picker is sending on. */
  const [forwardEntry, setForwardEntry] = useState<Entry | null>(null)
  /** Shared files open over the chat, each its own tab, and the one shown. */
  const [files, setFiles] = useState<OpenFile[]>(initialTabs.files)
  const [activeFile, setActiveFile] = useState<string | null>(initialTabs.activeFile)
  const [editingGroup, setEditingGroup] = useState(false)
  const [templateDraft, setTemplateDraft] = useState<BotDraft | null>(null)
  const [showSettings, setShowSettings] = useState(false)
  const [detailsTab, setDetailsTab] = useState<DetailsTab>('details')
  const [compactDetails, setCompactDetails] = useState(false)
  const [calling, setCalling] = useState(false)
  const [callSpeaking, setCallSpeaking] = useState(false)
  const interruptCall = useRef<(() => void) | null>(null)
  const [routineId, setRoutineId] = useState<string | null>(null)
  // A group member's settings, opened from its notice.
  const [editingMember, setEditingMember] = useState<string | null>(null)
  const [activeApp, setActiveApp] = useState<BotApp | null>(null)
  const [routineRequest, setRoutineRequest] = useState(0)
  // The artifacts shown over the chat (a card or the Library opened them), each its own tab.
  const [artifacts, setArtifacts] = useState<string[]>(initialTabs.artifacts)
  const [activeArtifact, setActiveArtifact] = useState<string | null>(initialTabs.activeArtifact)
  const root = useRef<HTMLDivElement>(null)
  const dock = useRef<HTMLDivElement>(null)
  // The width is measured, not rendered: resizing re-renders only when `wide` flips, or while the
  // window is narrow enough to squeeze the open side panel.
  const width = useRef(800)
  const [wide, setWide] = useState(true)
  const [, setSqueezed] = useState(0)
  const squeezeBelow = useRef(0)
  const [detailsWidth, setDetailsWidth] = usePref(prefs.detailsPanelWidth)
  useReading(store, botId)
  // The chat window remounts this per conversation (`key`), so each one opened counts.
  useScreenViewed('chat')

  useLayoutEffect(() => {
    const el = root.current
    if (!el) return
    const observer = new ResizeObserver(([e]) => {
      const w = e!.contentRect.width
      width.current = w
      setWide(w >= WIDE)
      setSqueezed(w - 380 < squeezeBelow.current ? w : 0)
    })
    observer.observe(el)
    // The composer floats over the transcript; its height becomes the transcript's bottom padding.
    const docked = new ResizeObserver(([e]) => el.style.setProperty('--composer-height', `${Math.ceil(e!.borderBoxSize[0]!.blockSize)}px`))
    if (dock.current) docked.observe(dock.current)
    return () => {
      observer.disconnect()
      docked.disconnect()
    }
  }, [])

  const panelOpen = wide && showSettings
  squeezeBelow.current = panelOpen ? Math.max(360, detailsWidth) : 0
  // The side panel's edge drags its width, leaving the conversation at least 380 wide.
  const panelWidth = (w: number) => Math.min(Math.max(360, w), width.current - 380)
  // A compact Details presentation dismisses once the window is wide enough for the inspector.
  useEffect(() => {
    if (wide) setCompactDetails(false)
  }, [wide])
  const toggleDetails = useCallback(() => {
    if (!wide) setCompactDetails((c) => !c)
    else setShowSettings((s) => !s)
  }, [wide])
  const devServers = devServerSummary(bot?.devServers ?? [])
  /** The agent's purpose on one line: OpenBot 0.70.0's title in the chat header. A group has none of its own. */
  const purpose = bot && !isGroup(bot) ? (bot.description.split('\n')[0] ?? '').trim() : ''
  // The title's port leads to its row, whichever page the inspector was left on.
  const openFromTitle = useCallback(() => {
    if (devServers.kind !== 'none') setDetailsTab('details')
    toggleDetails()
  }, [devServers.kind, toggleDetails])

  useEffect(
    () =>
      window.codync.app.onCommand((c) => {
        if (c.kind === 'toggleDetails') toggleDetails()
      }),
    [toggleDetails],
  )

  const openTrace = useCallback(() => setShowTrace(true), [])
  const openForward = useCallback((e: Entry) => setForwardEntry(e), [])
  const openFile = useCallback((entry: Entry, file: SharedFile) => {
    setFiles((list) => (list.some((item) => item.file.id === file.id) ? list : [...list, { entry, file, entryId: entry.id }]))
    setActiveFile(file.id)
  }, [])
  const closeFile = useCallback((id: string) => {
    setFiles((list) => {
      const at = list.findIndex((item) => item.file.id === id)
      const next = list.filter((item) => item.file.id !== id)
      setActiveFile((active) => (active !== id ? active : (next[Math.max(0, at - 1)]?.file.id ?? null)))
      return next
    })
  }, [])
  const openThread = useCallback((rootId: string) => setThreadRoot(rootId), [])
  // A thread branched from a chat message, then opened over the chat.
  const openBranch = useCallback((e: Entry) => {
    void store.startThread(botId, e.id).then(setThreadRoot)
  }, [store, botId])
  const openArtifact = useCallback((id: string) => {
    setArtifacts((list) => (list.includes(id) ? list : [...list, id]))
    setActiveArtifact(id)
    setCompactDetails(false)
  }, [])
  const openIslandMemory = useCallback(() => { setDetailsTab('memory'); setShowSettings(true); setCompactDetails(!wide) }, [wide])
  const openIslandOperation = useCallback((panel: 'trace' | 'computer' | 'documents', documentId?: string) => {
    if (panel === 'trace') { setShowTrace(true); return }
    setDetailsTab(panel); setShowSettings(true); setCompactDetails(!wide)
    if (panel === 'documents' && documentId) setRoutineRequest((request) => request + 1)
  }, [wide])
  const islandNavigation = useIslandThreadNavigation(`${store.computer.id}/${botId}`, openFile, openArtifact, openIslandMemory, openIslandOperation, setActiveApp, store)
  const closeArtifact = useCallback((id: string) => {
    setArtifacts((list) => {
      const next = list.filter((item) => item !== id)
      setActiveArtifact((active) => (active !== id ? active : (next[Math.max(0, list.indexOf(id) - 1)] ?? null)))
      return next
    })
  }, [])

  // While a chat is open this window beats its heartbeat, so a window opened beside it cannot adopt
  // the same workspace. After a restart there is no heartbeat and the last workspace is adopted.
  useEffect(() => {
    markWindowActive(windowId)
    const timer = setInterval(() => markWindowActive(windowId), 5_000)
    return () => clearInterval(timer)
  }, [windowId])

  // Every open, close or tab switch writes this chat's safe tab ids back to the device pref.
  useEffect(() => {
    const tabs = capTabs([
      ...files.flatMap((item) => {
        const tab = safeFileTab(item, chatRef)
        return tab ? [tab] : []
      }),
      ...artifacts.flatMap((id) => {
        const tab = safeArtifactTab(id)
        return tab ? [tab] : []
      }),
    ])
    const shown = activeArtifact
      ? safeArtifactTab(activeArtifact)
      : activeFile
        ? files.find((item) => item.file.id === activeFile) && safeFileTab(files.find((item) => item.file.id === activeFile)!, chatRef)
        : null
    const active = shown && tabs.some((tab) => sameTab(tab, shown)) ? shown : tabs[tabs.length - 1] ?? null
    const next = { ...prefs.openTabsByChat.get() }
    if (tabs.length) next[chatKey(chatRef)] = { window: windowId, ...chatRef, tabs, active }
    else delete next[chatKey(chatRef)]
    prefs.openTabsByChat.set(next)
  }, [files, artifacts, activeFile, activeArtifact, chatRef, windowId])

  const presentRoutine = useCallback((id: string | null) => {
    setRoutineId(id)
    setRoutineRequest((r) => r + 1)
    setDetailsTab('details')
    setShowSettings(true)
    if (!wide) setCompactDetails(true)
  }, [wide])

  // This bot's Agent card is on the Details tab; a group member's is in its settings.
  const presentAgent = useCallback((id: string) => (id === botId ? presentRoutine(null) : setEditingMember(id)), [botId, presentRoutine])
  const member = editingMember ? store.bots.get(editingMember) : undefined

  const details = (
    <DetailsPanel
      key={routineRequest}
      botId={botId}
      routineId={routineId}
      tab={detailsTab}
      setTab={setDetailsTab}
      editGroup={() => setEditingGroup(true)}
      openArtifact={openArtifact}
      share={() => {
        setCompactDetails(false)
        if (bot) setTemplateDraft(draftOf(bot))
      }}
      close={() => {
        setShowSettings(false)
        setCompactDetails(false)
      }}
    />
  )

  return (
    <div className="thread-view" ref={root}>
      <IslandBudget open={islandNavigation.budget} botId={botId} onClose={islandNavigation.closeBudget} />
      <IslandTerminal terminal={islandNavigation.terminal} onClose={islandNavigation.close} />
      <MessageContext botId={botId} />
      <div className="conversation">
        <div className="top-chrome">
          <TopFade />
          <div className="top-chrome-row">
            {!store.mismatch ? <MacChatButton className="ringed" title="Pinned messages" icon="pin" onClick={() => setShowPins(true)} /> : null}
            {bot && store.shownConnection.kind === 'online' ? (
              <button className="older-messages" title={tr('Older messages are still searchable')} onClick={() => setShowOlder(true)}>
                <Icon name="magnifyingglass" size={10} />
                <Text size="caption">{tr('Search older messages')}</Text>
              </button>
            ) : null}
            <Spacer />
            {(!showSettings || !wide) && !compactDetails ? (
              <ChromeIconButton ringed icon="chevron.left.2" label="Conversation details" onClick={toggleDetails} />
            ) : null}
          </div>
          {/* The call bar takes the pill's place; its frosted glass would show the name blurred. */}
          <button className={`title-pill ${calling ? 'hidden-by-call' : ''}`} inert={calling} onClick={openFromTitle} aria-label={tr('{name}, view details').replaceAll('{name}', () => bot?.name ?? tr('Conversation'))} title="View conversation details">
            <HStack as="span" gap={7}>
              {bot ? isGroup(bot) ? <GroupAvatar members={store.members(bot)} size={22} /> : <BotAvatar bot={bot} size={22} /> : null}
              <Text size="body" weight="medium" tone="text" truncate>{bot?.name ?? ''}</Text>
              {/* Ported from OpenBot 0.70.0: the agent's purpose after its name — Codync's `description` — gone once the pill is narrow. */}
              {purpose ? <Text as="span" size="body" tone="secondary" truncate className="title-pill-title">{purpose}</Text> : null}
              <DevServerTitle summary={devServers} />
              {/* Grok Bot's hint on hover: the pill opens the details. */}
              <span className="title-pill-chevron">
                <Icon name="chevron.right" size={10} weight="semibold" color="var(--secondary)" />
              </span>
            </HStack>
            {/* OpenBot nightly: with the island quiet, the chat keeps this small activity line by the
                avatar until the first text or step, from the host's own status (no new polling). */}
            {bot && isWorking(bot) ? <Text size="caption2" tone="secondary" truncate className="title-pill-activity">{tr(activityLine(bot))}</Text> : null}
            {store.shownConnection.kind !== 'online' || store.mismatch ? <ConnectionSubtitle /> : null}
          </button>
        </div>
        {/* An update card takes the composer's place, so there's nothing to reply with. */}
        <Transcript botId={botId} openTrace={openTrace} onReply={store.mismatch ? undefined : setReplyTo} openForward={store.mismatch ? undefined : openForward} openFile={openFile} openRoutine={presentRoutine} openAgent={presentAgent} openBotChat={setBotChat} openApp={setActiveApp} openArtifact={openArtifact} openCall={setCallLine} openThread={openThread} onBranch={store.mismatch ? undefined : openBranch} />
        <div className="composer-dock" ref={dock}>
          <div className="composer-fade" />
          {store.mismatch ? (
            <div className="composer-update-needed">
              <UpdateNeededCard />
            </div>
          ) : activeArtifact ? null : (
            // The artifact over the chat has its own box (two would both take the voice shortcut).
            <>
              <QueuedMessages botId={botId} />
              <PresenceRow botId={botId} />
              <Composer
                botId={botId}
                reply={replyTo}
                cancelReply={cancelReply}
                onCall={!calling && bot ? () => setCalling(true) : null}
                onInterrupt={callSpeaking ? () => interruptCall.current?.() : null}
              />
            </>
          )}
        </div>
        {calling ? (
          <div className="call-overlay">
            <CallView botId={botId} onSpeaking={setCallSpeaking} interrupt={interruptCall} onEnd={() => setCalling(false)} />
          </div>
        ) : null}
        {activeArtifact ? (
          <div className="artifact-tabbed">
            {artifacts.length > 1 ? (
              <TabStrip
                tabs={artifacts.map((id) => ({ id, title: store.artifacts.byId.get(id)?.title ?? tr('Artifact') }))}
                active={activeArtifact}
                label={tr('Open artifacts')}
                onSelect={setActiveArtifact}
                onClose={closeArtifact}
              />
            ) : null}
            <ArtifactView key={activeArtifact} botId={botId} artifactId={activeArtifact} close={() => closeArtifact(activeArtifact)} />
          </div>
        ) : null}
      </div>
      {activeFile ? (
        <div className="artifact-tabbed">
          {files.length > 1 ? (
            <TabStrip
              tabs={files.map(({ file }) => ({ id: file.id, title: file.name }))}
              active={activeFile}
              label={tr('Open files')}
              onSelect={setActiveFile}
              onClose={closeFile}
            />
          ) : null}
          {(() => {
            const shown = files.find((item) => item.file.id === activeFile)
            return shown ? <FileView key={shown.file.id} entryId={shown.entryId} file={shown.file} close={() => closeFile(shown.file.id)} /> : null
          })()}
        </div>
      ) : null}
      <ChatSidePanel
        open={panelOpen}
        width={panelWidth(detailsWidth)}
        min={360}
        max={width.current - 380}
        onResize={(w) => setDetailsWidth(panelWidth(w))}
      >
        {details}
      </ChatSidePanel>

      <Sheet open={compactDetails} onClose={() => setCompactDetails(false)} width={400} height={600}>
        {details}
      </Sheet>
      <Sheet open={botChat !== null} onClose={() => setBotChat(null)} width={620} height={560}>
        {botChat ? <BotConversationView botId={botId} peerId={botChat} /> : null}
      </Sheet>
      <Sheet open={forwardEntry !== null} onClose={() => setForwardEntry(null)} width={420} height={520}>
        {forwardEntry ? <ForwardSheet entry={forwardEntry} close={() => setForwardEntry(null)} /> : null}
      </Sheet>
      <Sheet open={threadRoot !== null} onClose={() => setThreadRoot(null)} width={620} height={560}>
        {threadRoot ? <RepliesView botId={botId} rootId={threadRoot} close={() => setThreadRoot(null)} /> : null}
      </Sheet>
      <Sheet open={callLine !== null} onClose={() => setCallLine(null)} width={620} height={560}>
        {callLine ? <CallConversationView botId={botId} lineId={callLine} /> : null}
      </Sheet>
      <Sheet open={showTrace} onClose={() => setShowTrace(false)} width={620} height={560}>
        <TraceView botId={botId} />
      </Sheet>
      <Sheet open={showPins} onClose={() => setShowPins(false)} width={460} height={520}>
        {showPins ? <PinsSheet botId={botId} close={() => setShowPins(false)} openThread={openThread} /> : null}
      </Sheet>
      <Sheet open={showOlder} onClose={() => setShowOlder(false)} width={560} height={620} dismissOnDim>
        <SheetPage>
          <ModalHeader title={tr('Older messages')} onClose={() => setShowOlder(false)} closeTitle={tr('Close')} />
          {bot ? (
            <RosterHistory
              items={[{ ref: { computerId: store.computer.id, botId }, bot, computer: store.computer, store }]}
              onPick={() => setShowOlder(false)}
              onClose={() => setShowOlder(false)}
            />
          ) : null}
        </SheetPage>
      </Sheet>
      <Sheet open={templateDraft !== null} onClose={() => setTemplateDraft(null)}>
        {templateDraft ? <BotTemplateView draft={templateDraft} /> : null}
      </Sheet>
      <Sheet open={editingGroup} onClose={() => setEditingGroup(false)} width={420} height={560}>
        {bot ? <GroupEditorView group={bot} /> : null}
      </Sheet>
      <FormSheet open={!!member} onClose={() => setEditingMember(null)} width={520} height={680} fixed>
        {member ? <BotEditorView key={member.id} draft={draftOf(member)} /> : null}
      </FormSheet>
      <BotAppPanel
        app={activeApp}
        onClose={() => setActiveApp(null)}
        onCreateShareLink={async (slug, label, mode) => {
          const shared = await store.createAppLink(botId, slug, mode, label)
          return shared
        }}
        onRevokeShareLink={async (slug, token) => {
          await store.revokeAppLink(botId, slug, token)
        }}
      />
    </div>
  )
})

/** The top edge of the chat: messages fade out gradually as they scroll under the title pill. */
function TopFade() {
  return <div className="top-fade" />
}

function ConnectionSubtitle() {
  const store = useStore()
  const tr = useTr()
  const connecting = store.shownConnection.kind === 'connecting'
  const symbol = store.connection.kind !== 'online' ? 'wifi.slash' : store.hostRoute === 'relay' ? 'cloud' : store.hostRoute === 'direct' ? 'wifi' : store.hostRoute === 'loopback' ? 'desktopcomputer' : 'network'
  const description = (() => {
    if (store.isAsleep) return 'Sleeping · wakes when you message'
    switch (store.shownConnection.kind) {
      case 'connecting':
        return 'Connecting'
      case 'computerOffline':
      case 'offline':
        return 'Offline'
      case 'unauthorized':
        return 'No access'
      case 'unpaired':
        return 'Not paired'
    }
    if (store.isCloudUpdating) return 'Updating…'
    if (store.mismatch) return 'Needs update'
    return store.hostRoute === 'relay' ? connectedThrough(store.computer.cloud) : store.hostRoute === 'direct' ? tr('Connected over Wi-Fi or Tailscale') : store.hostRoute === 'loopback' ? tr('Connected locally') : 'Connected'
  })()
  const spoken = tr(description)
  return (
    <HStack as="span" gap={6} className="title-pill-subtitle" title={`${store.hostName} · ${spoken}`} aria-label={`${store.hostName}, ${spoken}`}>
      <IconSlot width={12}>{connecting ? <Spinner size={8} /> : <Icon name={symbol} size={8} weight="medium" />}</IconSlot>
      <Text size="caption" weight="medium" truncate>{store.isAsleep ? description : store.connectionLabel}</Text>
    </HStack>
  )
}

/** The messages: every loaded message, brought to the newest as it changes while the reader is at the bottom. */
const PAGE = 40

const Transcript = memo(function Transcript({ botId, openTrace, onReply, openForward, openFile, openRoutine, openAgent, openBotChat, openApp, openArtifact, openCall, openThread, onBranch }: { botId: string; openTrace: () => void; onReply?: (e: Entry) => void; openForward?: (e: Entry) => void; openFile?: (e: Entry, file: SharedFile) => void; openRoutine: (id: string | null) => void; openAgent: (botId: string) => void; openBotChat: (peerId: string) => void; openApp?: (app: BotApp) => void; openArtifact: (artifactId: string) => void; openCall: (lineId: string) => void; openThread: (rootId: string) => void; onBranch?: (e: Entry) => void }) {
  const store = useStore()
  const tr = useTr()
  const bot = store.bots.get(botId) ?? null
  const live = !!bot && isWorkingIn(bot, botId) && !store.isOffline
  // Rebuilt only when this chat changes: the store replaces a chat's list on every change.
  const entries = store.chatEntries(botId)
  // What voice calls carried shows in each call's conversation, not in the chat; what waits for the bot, in the queue.
  const working = !!bot && isWorking(bot)
  const chatOf = useCallback((list: Entry[]) => buildChat(outsideQueue(hidingCalls(list), working)), [working])
  const all = useMemo(() => chatOf(entries), [chatOf, entries])
  const steps = useMemo(() => workingSteps(entries), [entries])
  const kofiAt = useKofiInviteAt(botId, all)
  // The newest 40 items first; older pages come in near the top.
  const [firstShown, setFirstShown] = useState<string | null>(null)
  const [loadingEarlier, setLoadingEarlier] = useState(false)
  const found = firstShown ? all.findIndex((i) => i.id === firstShown) : -1
  const start = found >= 0 ? found : Math.max(0, all.length - PAGE)
  const items = all.slice(start)
  const more = start > 0 || store.canLoadOlder(botId)
  // Following the newest message, or reading history: scrolling up releases the bottom,
  // arriving back at the end (or Jump to latest) restores it. Content growth alone never changes it.
  const [following, setFollowing] = useState(true)
  const followingRef = useRef(true)
  followingRef.current = following
  const scroller = useRef<HTMLDivElement>(null)
  const lastScrollTop = useRef(0)
  const keepPlace = useRef<number | null>(null)
  const last = items[items.length - 1]
  const jump = usePresence(!following && items.length > 0, 200)
  const arrivals = useArrivals([...items.map((i) => i.id), ...(live ? ['working'] : [])], following)
  const newCount = useNewMessageCount(items.map((i) => i.id), following)

  const toBottom = (smooth = false) => {
    const el = scroller.current
    if (el) el.scrollTo({ top: el.scrollHeight, behavior: smooth ? 'smooth' : 'auto' })
  }

  useEffect(() => {
    if (!firstShown && items[0]) setFirstShown(items[0].id)
  }, [firstShown, items])

  const showEarlier = async () => {
    if (loadingEarlier) return
    setLoadingEarlier(true)
    const el = scroller.current
    keepPlace.current = el ? el.scrollHeight - el.scrollTop : null
    let list = all
    let at = start
    if (at === 0) {
      const anchor = list[0]?.id
      await store.loadOlder(botId)
      list = chatOf(newestHealthNotices(store.chatEntries(botId)))
      at = anchor ? Math.max(0, list.findIndex((i) => i.id === anchor)) : 0
    }
    if (at > 0) setFirstShown(list[Math.max(0, at - PAGE)]!.id)
    setLoadingEarlier(false)
  }

  // Earlier messages going in above keep the reader's place; a follower stays at the bottom.
  useLayoutEffect(() => {
    const el = scroller.current
    if (!el) return
    if (keepPlace.current !== null && !followingRef.current) {
      el.scrollTop = el.scrollHeight - keepPlace.current
      keepPlace.current = null
    } else if (followingRef.current) toBottom()
  })

  // A message the user sends brings the chat back to the newest.
  const lastIsUser = last?.kind === 'entry' && last.entry.kind === 'user'
  useEffect(() => {
    if (lastIsUser) setFollowing(true)
  }, [last?.id, lastIsUser])

  // Rows that grow after layout (images, the text size) keep a follower at the bottom.
  useEffect(() => {
    const el = scroller.current
    if (!el) return
    const observer = new ResizeObserver(() => {
      if (followingRef.current) toBottom()
    })
    observer.observe(el.firstElementChild!)
    observer.observe(el)
    return () => observer.disconnect()
  }, [])

  return (
    <div className="transcript-frame">
      <div
        className="transcript"
        style={bot?.chatColor ? { background: `color-mix(in srgb, ${bot.chatColor} 10%, var(--background))` } : undefined}
        ref={scroller}
        role="log"
        aria-label={tr('Messages')}
        onScroll={(e) => {
          const el = e.currentTarget
          const atEnd = el.scrollHeight - el.scrollTop - el.clientHeight < 8
          if (el.scrollTop < lastScrollTop.current - 1 && !atEnd) setFollowing(false)
          else if (atEnd) setFollowing(true)
          lastScrollTop.current = el.scrollTop
          if (more && el.scrollTop < 200 && !loadingEarlier) void showEarlier()
        }}
      >
        <div className="transcript-content">
          {more ? (
            <button className="load-earlier" disabled={loadingEarlier} onClick={() => void showEarlier()}>
              {loadingEarlier ? <Spinner size={14} /> : tr('Load earlier messages')}
            </button>
          ) : null}
          {items.length === 0 && bot ? <div className="transcript-intro">{isGroup(bot) ? <GroupIntroCard group={bot} /> : <IntroCard bot={bot} />}</div> : null}
          {items.map((item) => (
            <div key={item.id} className={arrivals.has(item.id) ? rowIn(item) : undefined}>
              <Row item={item} chat={bot} openTrace={openTrace} onReply={onReply} openForward={openForward} openFile={openFile} openRoutine={openRoutine} openAgent={openAgent} openBotChat={openBotChat} openApp={openApp} openArtifact={openArtifact} openCall={openCall} openThread={openThread} onBranch={onBranch} />
              {item.id === kofiAt ? <KofiInviteCard /> : null}
            </div>
          ))}
          {bot ? <SharedWorkspaceNotice bot={bot} /> : null}
          {bot ? <WorkingTransition bot={bot} visible={live} steps={steps} /> : null}
          <div className="chat-end" />
        </div>
      </div>
      {jump.mounted ? (
        <div className={`jump-to-latest ${jump.shown ? 'shown' : ''}`} inert={!jump.shown}>
          {newCount > 0 ? <span className="jump-new-count">{newCount} new</span> : null}
          <MacChatButton
            title="Jump to latest"
            icon="arrow.down"
            direction={[0, 1]}
            onClick={() => {
              setFollowing(true)
              toBottom(true)
            }}
          />
        </div>
      ) : null}
    </div>
  )
})

function Row({ item, chat, openTrace, onReply, openForward, openFile, openRoutine, openAgent, openBotChat, openApp, openArtifact, openCall, openThread, onBranch }: { item: ChatItem; chat: Bot | null; openTrace: () => void; onReply?: (e: Entry) => void; openForward?: (e: Entry) => void; openFile?: (e: Entry, file: SharedFile) => void; openRoutine: (id: string | null) => void; openAgent: (botId: string) => void; openBotChat: (peerId: string) => void; openApp?: (app: BotApp) => void; openArtifact: (artifactId: string) => void; openCall: (lineId: string) => void; openThread: (rootId: string) => void; onBranch?: (e: Entry) => void }) {
  if (item.kind === 'separator') {
    return <TimeSeparator date={item.date} />
  }
  if (item.kind === 'exchanges') return <BotMessageRow exchanges={item.exchanges} open={openBotChat} />
  const e = item.entry
  if (e.kind === 'notice' && e.data.routineId) return <RoutineNotice entry={e} open={() => openRoutine(e.data.routineId!)} />
  if (e.kind === 'notice' && e.data.call) return <div className="chat-row notice"><VStack gap={8}><CallLineRow entry={e} open={() => openCall(e.id)} />{window.codync.platform === 'win32' && e.data.source === 'voice_receipt' ? <VoiceReceipt entry={e} /> : null}</VStack></div>

  return <ChatRow entry={e} groupStart={item.groupStart} chat={chat} openTrace={openTrace} onReply={onReply} onForward={openForward} onOpenFile={openFile} openAgent={openAgent} openApp={openApp} openArtifact={openArtifact} openThread={openThread} onBranch={onBranch} />
}

/** Warns when other bots are actively working in the same project workspace (N16). */
function SharedWorkspaceNotice({ bot }: { bot: Bot }) {
  const store = useStore()
  const peers = sharedWorkspacePeers(bot, store.bots.values())
  if (peers.length === 0) return null
  const names = peers.map((p) => p.name).join(', ')
  return (
    <HStack gap={6} justify="center" className="chat-row notice">
      <Icon name="exclamationmark.triangle" size={11} color="var(--warning, #ff9f0a)" />
      <Text size="caption" tone="attention">
        {`${names} ${peers.length === 1 ? 'is' : 'are'} also active in ${folderName(bot)}`}
      </Text>
    </HStack>
  )
}

/** The start of an empty group chat: who's in it and how the room works. */
function GroupIntroCard({ group }: { group: Bot }) {
  const tr = useTr()
  const store = useStore()
  const members = store.members(group)
  const names = members.map((m) => m.name)
  return (
    <div className="intro-card">
      <GroupAvatar members={members} size={72} />
      <Text as="div" size="title2" weight="semibold" tone="text">{group.name}</Text>
      {group.name !== names.join(', ') ? <Text as="div" size="subheadline" tone="secondary">{names.join(' · ')}</Text> : null}
      {group.description ? <Text as="div" size="subheadline" tone="secondary">{group.description}</Text> : null}
      <Text as="div" size="callout" tone="secondary" className="intro-card-hint balanced">{tr(`Everyone answers in turn. @mention a bot to ask just that one.`)}</Text>
    </div>
  )
}

function IntroCard({ bot }: { bot: Bot }) {
  const store = useStore()
  const tr = useTr()
  const where = bot.managedWorkspace
    ? `${store.backendName(bot.backend)} · ${tr('Personal workspace')}`
    : fill(tr('{agent} in {path}'), { agent: store.backendName(bot.backend), path: bot.cwd })
  return (
    <div className="intro-card">
      <BotAvatar bot={bot} size={72} />
      <Text as="div" size="title2" weight="semibold" tone="text">{bot.name}</Text>
      <Text as="div" size="callout" tone="secondary" className="intro-card-path">
        {where}
      </Text>
      {bot.description ? <Text as="div" size="subheadline" tone="secondary">{bot.description}</Text> : null}
      <Text as="div" size="callout" tone="secondary" className="intro-card-hint">{tr(`Tell it what you need. You'll get a notification when it's done or needs you.`)}</Text>
    </div>
  )
}
