import { execFile } from 'node:child_process'
import { BrowserWindow, globalShortcut, ipcMain, screen } from 'electron'
import type { WindowCommand } from '../shared/ipc'
import type { VoiceKey } from '../shared/voice-shortcut'
import {
  hitRect,
  hits,
  notchOf,
  type IslandPlacement,
  type IslandRect,
  type IslandScreen,
  type IslandSummary,
  type NotchScreen,
} from '../shared/island'
import { prefs } from './host-controller'
import { dockFor, islandPanelBounds, normalizeIslandControls, snapIslandDock, type IslandControls, type IslandDisplay } from '../shared/island-layout'
import { validIslandAction } from '../shared/island-actions'
import { IslandShortcuts } from './island-shortcuts'

// Every NSScreen's frame, safe-area top and the menu bar on each side of the notch. JXA reaches
// AppKit without a helper binary; it runs when the island opens and when displays change.
const SCREENS = `ObjC.import('AppKit'); JSON.stringify($.NSScreen.screens.js.map((s) => {
  const top = s.safeAreaInsets.top
  return { x: s.frame.origin.x, width: s.frame.size.width, top,
    left: top > 0 ? s.auxiliaryTopLeftArea.size.width : 0, right: top > 0 ? s.auxiliaryTopRightArea.size.width : 0 }
}))`

/** Linux on Wayland: a window can't place itself at the top of the screen or stay above others. */
export const islandSupported = process.platform !== 'linux' || process.env.XDG_SESSION_TYPE !== 'wayland'

/**
 * Linux (X11) only learns where the pointer is from its own windows' events, so it can't poll it
 * like macOS and Windows do: the window is shaped to the island instead, and the renderer sees
 * the pointer arrive.
 */
const shapedWindow = process.platform === 'linux'
/** A shrinking shape keeps its old outline until the close animation (0.34 s) is over. */
const SHRINK_MS = 400

function readScreens(): Promise<NotchScreen[]> {
  if (process.platform !== 'darwin') return Promise.resolve([])
  return new Promise((resolve) => {
    execFile('/usr/bin/osascript', ['-l', 'JavaScript', '-e', SCREENS], { timeout: 5000 }, (error, stdout) => {
      try {
        if (error) throw error
        resolve(JSON.parse(stdout) as NotchScreen[])
      } catch (e) {
        // Without the notch's size the island sits at the top edge like on a screen without one.
        console.error(`island: couldn't read the notch: ${e instanceof Error ? e.message : String(e)}`)
        resolve([])
      }
    })
  })
}

/** Answers the approval on screen from any app while it shows. */
const KEYS = { 'Alt+Y': 'allow', 'Alt+N': 'deny' } as const

interface Deps {
  url: string
  preload: string
  send: (command: WindowCommand, show?: boolean) => void
  openApp: () => void
  /** The setting changed (the tray's Settings menu shows it). */
  changed: () => void
}

/**
 * The island's transparent panel docks to a chosen display edge.
 * The renderer animates the shape inside it and reports where it is; the
 * window takes the mouse only over that shape and lets every other click through.
 */
export class Island {
  private window: BrowserWindow | null = null
  private summary: IslandSummary = { bots: [], permissions: [], usage: [], health: null }
  private screen: IslandScreen = { notch: null }
  private rect: IslandRect = { x: 0, y: 0, width: 0, height: 0 }
  private inside = false
  private timer: NodeJS.Timeout | null = null
  /** `follow`: the display the island is on, so it only moves when the pointer crosses to another one. */
  private placedOn = -1
  private watch: NodeJS.Timeout | null = null
  private shaped: IslandRect = { x: 0, y: 0, width: 0, height: 0 }
  private unfocus: NodeJS.Timeout | undefined
  private placing = 0
  private dragging = false
  private dragTimeout: NodeJS.Timeout | undefined
  private readonly shortcuts = new IslandShortcuts(globalShortcut, (action) => {
    if (action === 'quiet') this.configure({ quiet: !this.controls.quiet })
    else this.window?.webContents.send('island:shortcut', action)
  })

  constructor(private deps: Deps) {
    this.shortcuts.set(prefs.get('islandShortcuts'))
  }

  /** Off until turned on: the island is still experimental. */
  get enabled() {
    return islandSupported && prefs.get('showIsland') === true
  }

  setEnabled(on: boolean) {
    prefs.set('showIsland', on)
    this.sync()
    this.deps.changed()
    return this.enabled
  }

  /** Where the island sits on this computer's screens. */
  get placement(): IslandPlacement {
    return this.controls.placement
  }

  private get controls(): IslandControls {
    return normalizeIslandControls(prefs.get('islandControls') ?? { placement: prefs.get('islandPlacement') })
  }

  private configure(patch: Partial<IslandControls>) {
    const next = normalizeIslandControls({ ...this.controls, ...patch })
    prefs.set('islandControls', next)
    prefs.set('islandPlacement', next.placement)
    this.publishControls()
    void this.place()
    this.deps.changed()
    return next
  }

  setPlacement(placement: IslandPlacement) {
    this.configure({ placement, displayId: null })
    return this.placement
  }

  register() {
    ipcMain.on('island:resourceOpened', () => { this.deps.openApp(); this.window?.webContents.send('island:resourceOpened') })
    ipcMain.on('island:draft', (_e, draft: { ref?: unknown; text?: unknown }) => {
      if (draft && typeof draft.ref === 'string' && typeof draft.text === 'string' && draft.text.length <= 1_000_000) this.window?.webContents.send('island:draft', { ref: draft.ref, text: draft.text })
    })
    ipcMain.handle('island:shortcuts', () => this.shortcuts.state)
    ipcMain.handle('island:setShortcuts', (_e, input: unknown) => {
      const state = this.shortcuts.set(input)
      prefs.set('islandShortcuts', state.bindings)
      this.publishControls()
      return state
    })
    ipcMain.handle('island:configure', (_e, patch: Partial<IslandControls>) => this.configure(patch))
    ipcMain.on('island:action', (_e, input: unknown) => {
      const action = validIslandAction(input)
      if (action) this.deps.send({ kind: 'islandAction', action }, ['openFlow', 'openChannels', 'openDiagnostics'].includes(action.kind))
    })
    ipcMain.on('island:drag', (_e, on: boolean, snap: boolean) => this.drag(on, snap))
    ipcMain.on('island:supported', (e) => (e.returnValue = islandSupported))
    ipcMain.handle('island:enabled', () => this.enabled)
    ipcMain.handle('island:setEnabled', (_e, on: boolean) => this.setEnabled(on))
    ipcMain.handle('island:state', () => ({ screen: this.screen, summary: this.summary }))
    ipcMain.on('island:summary', (_e, summary: IslandSummary) => {
      this.summary = summary
      this.window?.webContents.send('island:summary', summary)
    })
    ipcMain.on('island:shape', (_e, rect: IslandRect) => {
      this.rect = rect
      if (shapedWindow) this.shape()
    })
    ipcMain.on('island:respond', (_e, ref: string, entryId: string, optionId: string) =>
      this.deps.send({ kind: 'respondPermission', ref, entryId, optionId }, false),
    )
    ipcMain.on('island:openBot', (_e, ref: string) => this.deps.send({ kind: 'openBot', ref }))
    ipcMain.on('island:send', (_e, ref: string, text: string) => this.deps.send({ kind: 'sendMessage', ref, text }, false))
    // Files dropped on the notch: the chat window reads them and sends them to that bot.
    ipcMain.on('island:dropFiles', (_e, ref: string, paths: string[]) => {
      if (!Array.isArray(paths) || paths.length === 0) return
      this.deps.send({ kind: 'sendFiles', ref, paths: paths.filter((p) => typeof p === 'string').slice(0, 20) }, false)
    })
    ipcMain.on('island:newBot', () => this.deps.send({ kind: 'newBot' }))
    ipcMain.on('island:openApp', () => this.deps.openApp())
    ipcMain.on('island:focusable', (_e, on: boolean) => this.setFocusable(on))
    ipcMain.on('island:keys', (_e, on: boolean) => this.setKeys(on))
    screen.on('display-added', () => void this.place())
    screen.on('display-removed', () => void this.place())
    screen.on('display-metrics-changed', () => void this.place())
    this.sync()
  }

  private sync() {
    if (this.enabled) this.open()
    else this.close()
  }

  private open() {
    if (this.window) return
    const isMac = process.platform === 'darwin'
    // Apply the saved safe dock before showing; notch measurements may arrive later.
    const display = this.target([])
    const controls = this.controls
    const dock = dockFor(controls, display.id)
    const initial = islandPanelBounds(display, dock, false)
    this.screen = {
      notch: null, edge: dock.edge, density: controls.density,
      panel: { width: initial.width, height: initial.height },
      controls, displayId: display.id, displays: this.displays(),
      quiet: this.effectiveQuiet(), shortcutState: this.shortcuts.state,
    }
    const win = new BrowserWindow({
      ...initial,
      show: false,
      frame: false,
      transparent: true,
      backgroundColor: '#00000000',
      hasShadow: false,
      resizable: false,
      movable: false,
      minimizable: false,
      maximizable: false,
      fullscreenable: false,
      focusable: false,
      skipTaskbar: true,
      // Lets the panel cover the menu bar.
      enableLargerThanScreen: true,
      acceptFirstMouse: true,
      // macOS: a non-activating panel on every Space and over full-screen apps; Windows: out of Alt+Tab.
      ...(isMac ? { type: 'panel' } : process.platform === 'win32' ? { type: 'toolbar' } : {}),
      webPreferences: { preload: this.deps.preload, sandbox: true, contextIsolation: true },
    })
    // Above the menu bar on macOS (main menu + 3).
    win.setAlwaysOnTop(true, isMac ? 'main-menu' : 'screen-saver', isMac ? 3 : 0)
    if (!isMac) win.setVisibleOnAllWorkspaces(true)
    // Takes no mouse until the renderer reports the shape.
    this.rect = { x: 0, y: 0, width: 0, height: 0 }
    this.shaped = this.rect
    if (shapedWindow) win.setShape([this.shaped])
    else win.setIgnoreMouseEvents(true)
    win.once('ready-to-show', () => win.showInactive())
    win.on('closed', () => {
      if (this.window === win) {
        this.window = null
        this.close()
      }
    })
    this.window = win
    this.shortcuts.activate()
    this.publishControls()
    this.inside = false
    void this.place()
    void win.loadURL(this.deps.url)
    if (!shapedWindow) this.tick()
    // A few times a second is enough to notice the pointer on another screen.
    this.watch = setInterval(() => {
      this.followPointer()
      const quiet = this.effectiveQuiet()
      if (quiet !== this.screen.quiet) {
        this.screen = { ...this.screen, quiet }
        this.window?.webContents.send('island:screen', this.screen)
      }
    }, 400)
  }

  /** `follow`: move the island to the screen the person is working on. */
  private followPointer() {
    if (!this.window || this.dragging || this.placement !== 'follow' || this.controls.displayId !== null) return
    const display = screen.getDisplayNearestPoint(screen.getCursorScreenPoint())
    if (display.id !== this.placedOn) void this.place()
  }

  private close() {
    this.shortcuts.deactivate()
    this.dragging = false
    clearTimeout(this.dragTimeout)
    ++this.placing
    this.setKeys(false)
    clearTimeout(this.unfocus)
    if (this.timer) clearTimeout(this.timer)
    if (this.watch) clearInterval(this.watch)
    this.timer = null
    this.watch = null
    this.placedOn = -1
    const win = this.window
    this.window = null
    win?.destroy()
  }

  /** Explicit keyboard interaction focuses the panel without bringing the main app forward. */
  private setFocusable(on: boolean) {
    const win = this.window
    if (!win) return
    clearTimeout(this.unfocus)
    win.setFocusable(on)
    if (on) win.focus()
    else if (win.isFocused()) {
      // Giving the keyboard back takes the window off screen for a moment on macOS (and to the back
      // of its level): wait until the shape has folded, then bring it forward again.
      this.unfocus = setTimeout(() => {
        if (!win.isDestroyed() && win.isFocused()) {
          win.blur()
          win.moveTop()
        }
      }, SHRINK_MS)
    }
  }

  private setKeys(on: boolean) {
    for (const [accelerator, answer] of Object.entries(KEYS)) {
      if (!on) {
        globalShortcut.unregister(accelerator)
        continue
      }
      if (globalShortcut.isRegistered(accelerator)) continue
      if (!globalShortcut.register(accelerator, () => this.window?.webContents.send('island:key', answer))) {
        console.error(`island: ${accelerator} is taken by another app`)
      }
    }
  }

  /** The voice input shortcut's taps, while the island is on. */
  voiceKey(key: VoiceKey) {
    this.window?.webContents.send('island:voiceKey', key)
  }

  /** Apply the chosen display's saved dock and usable work area. */
  private async place() {
    if (!this.window) return
    const generation = ++this.placing
    const screens = await readScreens()
    const win = this.window
    if (!win || win.isDestroyed() || generation !== this.placing) return
    const display = this.target(screens)
    this.placedOn = display.id
    const controls = this.controls
    const dock = dockFor(controls, display.id)
    const notch = dock.edge === 'top' && dock.offset === 0.5 ? notchOf(screens, display.bounds) : null
    const bounds = islandPanelBounds(display, dock, !!notch)
    this.screen = { notch, edge: dock.edge, density: controls.density, panel: { width: bounds.width, height: bounds.height }, controls, displayId: display.id, displays: this.displays() }
    this.screen = { ...this.screen, quiet: this.effectiveQuiet(), shortcutState: this.shortcuts.state }
    win.setBounds(bounds)
    win.webContents.send('island:screen', this.screen)
  }

  private displays(): IslandDisplay[] {
    return screen.getAllDisplays().map((d) => ({ id: d.id, label: d.label || `Display ${d.id}`, bounds: d.bounds, workArea: d.workArea }))
  }

  /** Fullscreen detection is limited to Codync's own windows; other apps use manual quiet mode. */
  private effectiveQuiet() {
    const controls = this.controls
    return controls.quiet || controls.quietFullscreen && BrowserWindow.getAllWindows().some((win) => win !== this.window && !win.isDestroyed() && win.isFullScreen())
  }

  private publishControls() {
    this.screen = { ...this.screen, controls: this.controls, quiet: this.effectiveQuiet(), shortcutState: this.shortcuts.state }
    this.window?.webContents.send('island:screen', this.screen)
  }

  private drag(on: boolean, snap: boolean) {
    const win = this.window
    if (!win) return
    clearTimeout(this.dragTimeout)
    if (on) {
      if (this.controls.locked) return
      this.dragging = true
      if (shapedWindow) {
        const { width, height } = win.getBounds()
        win.setShape([{ x: 0, y: 0, width, height }])
      }
      else win.setIgnoreMouseEvents(false)
      this.dragTimeout = setTimeout(() => this.drag(false, false), 15_000)
      return
    }
    if (!this.dragging) return
    this.dragging = false
    if (snap && !this.controls.locked) {
      const point = screen.getCursorScreenPoint()
      const display = screen.getDisplayNearestPoint(point)
      const first = snapIslandDock(point, display, win.getBounds())
      const bounds = islandPanelBounds(display, first, false)
      const dock = snapIslandDock(point, display, bounds)
      this.configure({ displayId: display.id, docks: { ...this.controls.docks, [display.id]: dock } })
    }
    if (shapedWindow) this.shape()
    else { this.inside = true; win.setIgnoreMouseEvents(false) }
  }

  private target(screens: NotchScreen[]) {
    const selected = screen.getAllDisplays().find((d) => d.id === this.controls.displayId)
    if (selected) return selected
    switch (this.placement) {
      case 'follow':
        return screen.getDisplayNearestPoint(screen.getCursorScreenPoint())
      case 'primary':
        return screen.getPrimaryDisplay()
      default:
        return screen.getAllDisplays().find((d) => notchOf(screens, d.bounds)) ?? screen.getPrimaryDisplay()
    }
  }

  /** Linux: the window draws and takes the mouse only over the island. */
  private shape() {
    const win = this.window
    if (!win) return
    const hit = hitRect(this.rect)
    const bounds = win.getBounds()
    const next = { x: Math.min(hit.x, bounds.width), y: Math.min(hit.y, bounds.height), width: Math.max(0, Math.min(hit.width, bounds.width - hit.x)), height: Math.max(0, Math.min(hit.height, bounds.height - hit.y)) }
    const grows = next.width * next.height >= this.shaped.width * this.shaped.height
    if (this.timer) clearTimeout(this.timer)
    const apply = () => {
      this.shaped = next
      if (!win.isDestroyed()) win.setShape([next])
    }
    if (grows) apply()
    else this.timer = setTimeout(apply, SHRINK_MS)
  }

  /** macOS and Windows: follows the pointer, 60 Hz near the panel and a few times a second elsewhere. */
  private tick = () => {
    const win = this.window
    if (!win || win.isDestroyed()) return
    const point = screen.getCursorScreenPoint()
    const bounds = win.getBounds()
    const x = point.x - bounds.x
    const y = point.y - bounds.y
    const inside = hits(this.rect, x, y)
    if (!this.dragging && inside !== this.inside) {
      this.inside = inside
      win.setIgnoreMouseEvents(!inside)
      win.webContents.send('island:hover', inside)
    }
    const near = x > -60 && x < bounds.width + 60 && y > -60 && y < bounds.height + 60
    this.timer = setTimeout(this.tick, near ? 16 : 150)
  }
}
