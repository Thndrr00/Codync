import {
  FINISHED_MS, focusOf, isActive, islandFrame, islandMode, othersOf, stopsDictation, transitions,
  type IslandBot, type IslandNotice, type IslandPermission, type IslandScreen, type IslandSummary, type IslandTab,
} from '@shared/island'
import { VOICE_SHORTCUT_KEYS, type VoiceKey, type VoiceShortcut } from '@shared/voice-shortcut'
import { Observable } from '../../lib/observable'
import { pref } from '../../lib/prefs'
import { Dictation } from '../thread/dictation'
import { dictationPlatform } from '../thread/dictation-platform'
import { defaultIslandControls, dockFor, localIslandRect, type IslandControls, type IslandEdge } from '@shared/island-layout'
import { usageNotice, type IslandAction } from '@shared/island-activity'
import { normalizeIslandHistory, updateIslandHistory, type IslandHistory } from '@shared/island-workspace'
import { islandUsage } from './island-preferences'
import type { IslandShortcutAction } from '@shared/island-shortcuts'

/** The bare notch peeks at once and opens after a rest; the compact island opens sooner. */
const OPEN_FROM_PEEK_MS = 650
const OPEN_FROM_COMPACT_MS = 200
const UNPEEK_MS = 600
/** Opened by hover, it folds soon after the pointer leaves; opened by a click, after a countdown. */
const LEAVE_MS = 800
export const PINNED_LEAVE_MS = 10_000
/** Set once the voice input shortcut has been used; until then the message field shows how. */
const savedHistory = pref<IslandHistory>('islandNoticeHistory', { items: [], active: [] })
const voiceShortcutUsed = pref('islandVoiceShortcutUsed', false)

/** The island window's state: the chat window's summary, the pointer, the header's tab and what was put off. */
export class IslandModel extends Observable {
  history = normalizeIslandHistory(savedHistory.get())
  summary: IslandSummary = { bots: [], permissions: [], usage: [], health: null }
  screen: IslandScreen = { notch: null }
  open = false
  tab: IslandTab = 'auto'
  notice: IslandNotice | null = null
  /** Counting down to fold after the pointer left a pinned island. */
  closing = false
  /** The bot the message field writes to. */
  messageRef: string | null = null
  /** The message field's text. */
  draft = ''
  private resumedDraft: { ref: string; original: string } | null = null
  /** Talking into the message field (the field's mic, or the voice input shortcut). */
  readonly dictation = new Dictation((change) => this.update(() => (this.draft = change(this.draft))), dictationPlatform())
  /** The voice input shortcut, as Settings → General has it. */
  shortcut: VoiceShortcut = 'off'
  /** The shape grew on its last change: open with a spring, close without one. */
  growing = true
  selectedRef: string | null = null
  approvalId: string | null = null
  configurationError: string | null = null
  actionError: string | null = null
  moving = false
  private peek = false
  private pinned = false
  private hovering = false
  private deferred = new Set<string>()
  private statuses: Map<string, string> | null = null
  private hoverTimer: ReturnType<typeof setTimeout> | undefined
  private noticeTimer: ReturnType<typeof setTimeout> | undefined
  private shape = { x: 0, y: 0, width: 0, height: 0 }
  private keys = false
  /** Manual controls can take the keyboard; it returns when the island closes. */
  private keyboard = false

  start() {
    const island = window.codync.island
    island.onSummary((s) => this.apply(s))
    island.onScreen((s) => this.update(() => (this.screen = s)))
    window.addEventListener('resize', () => this.update(() => {}))
    setInterval(() => this.update(() => {}), 30_000)
    island.onHover((inside) => this.hover(inside))
    island.onKey((answer) => this.answerByKey(answer))
    island.onVoiceKey((key) => this.voiceKey(key))
    island.onShortcut((action) => this.invokeShortcut(action))
    island.onResourceOpened(() => this.close())
    island.onDraft(({ ref, text }) => {
      if (!this.bot(ref)) return
      if (this.draft.trim()) { this.update(() => { this.actionError = 'Send or clear the current island message before resuming a saved draft.' }); return }
      this.update(() => { this.messageRef = ref; this.draft = text; this.resumedDraft = { ref, original: text }; this.actionError = null })
      this.setTab('message')
    })
    window.addEventListener('blur', () => {
      if (this.tab === 'message') this.close()
    })
    // Linux shapes the window to the island (main/island.ts): the pointer entering the window is the hover.
    if (window.codync.platform === 'linux') {
      document.documentElement.addEventListener('mouseenter', () => this.hover(true))
      document.documentElement.addEventListener('mouseleave', () => this.hover(false))
    }
    void island.state().then(({ screen, summary }) => {
      this.screen = screen
      this.apply(summary)
    })
  }

  get mode() {
    return islandMode(this.summary, { open: this.open, peek: this.peek, tab: this.tab, deferred: this.deferred, notice: this.screen.quiet ? null : this.notice })
  }

  get frame() {
    return islandFrame(this.mode, this.screen)
  }

  /** The approval to answer now: the first not put off, else the first. */
  get permission(): IslandPermission | null {
    const cards = this.summary.permissions
    return cards.find((p) => p.entryId === this.approvalId) ?? cards.find((p) => !this.deferred.has(p.entryId)) ?? cards[0] ?? null
  }

  get focus() {
    return this.bot(this.selectedRef) ?? focusOf(this.summary.bots)
  }

  get controls() { return this.screen.controls ?? defaultIslandControls }
  get rect() { return localIslandRect(this.frame, window.innerWidth, window.innerHeight, this.screen.edge ?? 'top') }

  selectFocus(ref: string) { this.update(() => { this.selectedRef = ref }) }

  nextFocus() {
    const bots = this.summary.bots
    if (!bots.length) return
    const at = bots.findIndex((bot) => bot.ref === this.focus?.ref)
    this.selectFocus(bots[(at + 1) % bots.length]!.ref)
  }

  async openN8n() {
    this.update(() => { this.actionError = null })
    try { await window.codync.n8n.open() }
    catch (error) { this.update(() => { this.actionError = error instanceof Error ? error.message : String(error) }) }
  }

  nextApproval(step: number) {
    const cards = this.summary.permissions
    if (!cards.length) return
    const at = cards.findIndex((card) => card.entryId === this.permission?.entryId)
    this.update(() => { this.approvalId = cards[(at + step + cards.length) % cards.length]!.entryId })
  }

  action(action: IslandAction) {
    window.codync.island.action(action)
    if (['openFlow', 'openChannels', 'openDiagnostics'].includes(action.kind)) this.close()
  }

  clearHistory() {
    this.history = { items: [], active: this.history.active }
    savedHistory.set(this.history)
    this.changed()
  }

  async setShortcuts(bindings: unknown) {
    try {
      const shortcutState = await window.codync.island.setShortcuts(bindings)
      this.update(() => { this.screen = { ...this.screen, shortcutState }; this.configurationError = null })
    } catch (error) { this.update(() => { this.configurationError = error instanceof Error ? error.message : String(error) }) }
  }

  private invokeShortcut(action: IslandShortcutAction) {
    if (action === 'toggle') { if (this.open) this.close(); else this.openNow() }
    else if (action !== 'quiet') this.setTab(action === 'search' ? 'workspace' : action)
  }

  async configure(patch: Partial<IslandControls>) {
    try {
      const controls = await window.codync.island.configure(patch)
      this.update(() => { this.screen = { ...this.screen, controls }; this.configurationError = null })
    } catch (error) {
      this.update(() => { this.configurationError = error instanceof Error ? error.message : String(error) })
    }
  }

  setEdge(edge: IslandEdge) {
    const id = this.screen.displayId
    if (id === undefined) return
    const dock = dockFor(this.controls, id)
    void this.configure({ docks: { ...this.controls.docks, [id]: { ...dock, edge } } })
  }

  drag(on: boolean, snap = false) {
    if (on && this.controls.locked) return
    this.update(() => { this.moving = on })
    window.codync.island.drag(on, snap)
  }

  /** The bots beside the big slot: pills (overview), the mini column and the compact cluster. */
  others(limit: number) {
    return othersOf(this.summary.bots, this.focus, limit)
  }

  get activeCount() {
    return this.summary.bots.filter(isActive).length
  }

  bot(ref: string | null) {
    return this.summary.bots.find((b) => b.ref === ref) ?? null
  }

  get noticeBot(): IslandBot | null {
    return this.bot(this.notice?.ref ?? null)
  }

  get messageBot(): IslandBot | null {
    return this.bot(this.messageRef) ?? this.focus
  }

  /** How to talk from any app, under the message field until the shortcut is first used. */
  get voiceTip() {
    const key = VOICE_SHORTCUT_KEYS[this.shortcut]
    return key && !voiceShortcutUsed.get() ? `Tip: double-tap ${key} to talk from any app, then tap it once to send.` : null
  }

  /** A click on the closed island opens it and keeps it open. */
  openNow() {
    this.keyboard = true
    window.codync.island.setFocusable(true)
    this.update(() => {
      this.open = true
      this.pinned = true
      this.peek = false
    })
    this.foldWhenAway()
  }

  /** The header's tabs. Bots puts the approvals on screen off until another arrives. */
  setTab(tab: IslandTab) {
    if (stopsDictation(this.tab, tab, this.dictation.busy)) this.dictation.finish()
    this.update(() => {
      if (tab === 'bots') for (const p of this.summary.permissions) this.deferred.add(p.entryId)
      if (tab === 'message' && !this.messageRef) this.messageRef = this.focus?.ref ?? null
      this.tab = tab
      this.open = true
      this.pinned = true
    })
    // Explicit tabs take the keyboard for their controls. Keep it through tab changes:
    // giving it back between them blinks the panel on macOS.
    if (tab !== 'auto') {
      this.keyboard = true
      window.codync.island.setFocusable(true)
      if (tab === 'message') this.readShortcut()
    }
    this.foldWhenAway()
    if (tab === 'activity' || tab === 'status') this.action({ kind: 'refreshActivity' })
  }

  /** The message field's bot chip steps through the bots. */
  nextMessageBot() {
    const bots = this.summary.bots
    if (!bots.length) return
    const at = bots.findIndex((b) => b.ref === this.messageBot?.ref)
    this.update(() => (this.messageRef = bots[(at + 1) % bots.length]!.ref))
  }

  setDraft(text: string) {
    this.update(() => (this.draft = text))
  }

  send() {
    const bot = this.messageBot
    if (!bot || !this.draft.trim()) return
    if (this.resumedDraft?.ref === bot.ref) this.action({ kind: 'sendSavedDraft', ref: bot.ref, text: this.draft, original: this.resumedDraft.original })
    else window.codync.island.send(bot.ref, this.draft)
    this.close()
  }

  respond(card: IslandPermission, optionId: string) {
    window.codync.island.respond(card.ref, card.entryId, optionId)
    if (this.open && this.summary.permissions.length <= 1) this.close()
    else this.nextApproval(1)
  }

  openBot(ref: string) {
    window.codync.island.openBot(ref)
    this.dismissNotice()
    this.close()
  }

  newBot() {
    window.codync.island.newBot()
    this.close()
  }

  openApp() {
    window.codync.island.openApp()
    this.close()
  }

  dismissNotice() {
    clearTimeout(this.noticeTimer)
    this.update(() => (this.notice = null))
  }

  close() {
    clearTimeout(this.hoverTimer)
    this.dictation.cancel()
    this.update(() => {
      this.draft = ''
      this.resumedDraft = null
      this.open = false
      this.pinned = false
      this.peek = false
      this.closing = false
      this.tab = 'auto'
    })
    if (this.keyboard) window.codync.island.setFocusable(false)
    this.keyboard = false
  }

  /** A double tap opens the message field and listens; the next tap sends what was said once the last words are in. */
  private voiceKey(key: VoiceKey) {
    if (!this.dictation.shortcutKey(key, () => this.send())) return
    voiceShortcutUsed.set(true)
    this.setTab('message')
    if (this.messageBot) this.dictation.start(this.draft, true)
  }

  private readShortcut() {
    void window.codync.speech.shortcut().then((shortcut) => this.update(() => (this.shortcut = shortcut)))
  }

  private answerByKey(answer: 'allow' | 'deny') {
    const card = this.mode === 'approval' ? this.permission : null
    const option = card?.options.find((o) => o.kind === (answer === 'allow' ? 'allow_once' : 'reject_once'))
    if (card && option && card.answering === null) this.respond(card, option.id)
  }

  private apply(summary: IslandSummary) {
    const changes = this.statuses ? transitions(this.statuses, summary.bots) : []
    this.statuses = new Map(summary.bots.map((b) => [b.ref, b.status]))
    this.update(() => {
      const notices = changes.map((notice) => ({ key: `${notice.kind}:${notice.ref}`, ref: notice.ref, text: `${summary.bots.find((bot) => bot.ref === notice.ref)?.name ?? notice.ref} · ${notice.kind}` }))
      for (const bot of summary.bots) if (bot.error) notices.push({ key: `bot:${bot.ref}:${bot.error}`, ref: bot.ref, text: `${bot.name} · ${bot.error}` })
      for (const error of [summary.health, summary.activity?.error, summary.activity?.refreshError, summary.workspace?.error, summary.workspace?.actionError]) if (error) notices.push({ key: `error:${error}`, ref: '', text: error })
      const usage = usageNotice(summary.usage, islandUsage.get(), Date.now())
      if (usage) notices.push({ key: `usage:${usage.kind}:${usage.provider}:${usage.resetsAt}`, ref: '', text: `${usage.provider} · ${usage.kind === 'limit' ? `${usage.percent}%` : 'Reset within 15 minutes'}` })
      const history = updateIslandHistory(this.history, notices, Date.now())
      if (JSON.stringify(history) !== JSON.stringify(this.history)) { this.history = history; savedHistory.set(history) }
      this.summary = summary
      // Answered and expired cards leave the put-off set.
      for (const id of this.deferred) if (!summary.permissions.some((p) => p.entryId === id)) this.deferred.delete(id)
      // A failed bot that works again no longer needs the notice.
      const shown = this.notice ? summary.bots.find((b) => b.ref === this.notice?.ref) : undefined
      if (shown && isActive(shown)) this.notice = null
      const next = changes.at(-1)
      if (next) this.showNotice(next)
    })
  }

  /** Done fades after a moment; failed stays until dismissed. */
  private showNotice(notice: IslandNotice) {
    clearTimeout(this.noticeTimer)
    this.notice = notice
    if (notice.kind === 'finished') this.noticeTimer = setTimeout(() => this.update(() => (this.notice = null)), FINISHED_MS)
  }

  private hover(inside: boolean) {
    this.hovering = inside
    clearTimeout(this.hoverTimer)
    const mode = this.mode
    if (inside) {
      if (this.screen.quiet && !this.open) return
      if (this.closing) this.update(() => (this.closing = false))
      if (mode === 'hidden') {
        this.update(() => (this.peek = true))
        this.hoverTimer = setTimeout(() => this.openByHover(), OPEN_FROM_PEEK_MS)
      } else if (mode === 'peek' || mode === 'compact') {
        this.hoverTimer = setTimeout(() => this.openByHover(), OPEN_FROM_COMPACT_MS)
      }
      return
    }
    if (this.open) {
      if (this.pinned) this.foldWhenAway()
      else this.hoverTimer = setTimeout(() => this.close(), LEAVE_MS)
    } else if (this.peek) {
      this.hoverTimer = setTimeout(() => this.update(() => (this.peek = false)), UNPEEK_MS)
    }
  }

  /**
   * Kept open with the pointer elsewhere (it left, or a click came through accessibility): fold
   * after a countdown. The message field stays until it sends, Esc or a click in another app.
   */
  private foldWhenAway() {
    clearTimeout(this.hoverTimer)
    const counting = this.open && !this.hovering && this.tab !== 'message' && this.tab !== 'options' && this.tab !== 'workspace' && !this.moving
    if (counting !== this.closing) this.update(() => (this.closing = counting))
    if (counting) this.hoverTimer = setTimeout(() => this.close(), PINNED_LEAVE_MS)
  }

  private openByHover() {
    if (!this.hovering || this.screen.quiet) return
    this.update(() => {
      this.open = true
      this.peek = false
    })
  }

  private update(change: () => void) {
    change()
    const frame = this.frame
    const rect = this.rect
    if (rect.x !== this.shape.x || rect.y !== this.shape.y || frame.width !== this.shape.width || frame.height !== this.shape.height) {
      this.growing = frame.width * frame.height >= this.shape.width * this.shape.height
      this.shape = rect
      window.codync.island.setShape(rect)
    }
    const keys = this.mode === 'approval'
    if (keys !== this.keys) {
      this.keys = keys
      window.codync.island.setKeys(keys)
    }
    this.changed()
  }
}
