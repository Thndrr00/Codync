import { app, ipcMain, BrowserWindow, type IpcMainInvokeEvent } from 'electron'
import { join } from 'node:path'
import type { ScreenView, ScreenWindowTarget } from '../shared/screen-viewer'
import { bridgeVnc } from './vnc-bridge'

interface Live {
  vncPort: number
  view: ScreenView
  close: () => void
  window: BrowserWindow
}

const live = new Map<string, Live>()

export function closeComputerScreen(computerId: string) {
  const current = live.get(computerId)
  if (!current) return
  live.delete(computerId)
  current.close()
  if (!current.window.isDestroyed()) current.window.destroy()
}

function rendererLocation(raw: string): URL {
  const url = new URL(raw)
  const loopback = ['localhost', '127.0.0.1', '[::1]'].includes(url.hostname)
  if (url.username || url.password || url.hash ||
      !(url.protocol === 'file:' || (!app.isPackaged && loopback && ['http:', 'https:'].includes(url.protocol)))) {
    throw new Error('The screen renderer must be a local application page.')
  }
  return url
}

function authorize(event: IpcMainInvokeEvent, window: BrowserWindow | null, location: URL) {
  if (!window || window.isDestroyed() || event.sender !== window.webContents ||
      event.senderFrame !== window.webContents.mainFrame ||
      rendererLocation(event.senderFrame.url).href !== location.href) {
    throw new Error('This window cannot access the remote screen.')
  }
}

/** Resolve targets from main-process tunnel state; renderer-supplied names and ports are ignored. */
export function registerScreenViewer(rendererURL: (page: string) => string, chatWindow: () => BrowserWindow | null,
  resolveTarget: (computerId: string) => ScreenWindowTarget | null) {
  const chatLocation = rendererLocation(rendererURL('chat'))
  const screenLocation = rendererLocation(rendererURL('screen'))
  const pending = new Set<string>()
  ipcMain.handle('screen:open', async (event, request: ScreenWindowTarget) => {
    authorize(event, chatWindow(), chatLocation)
    if (typeof request?.computerId !== 'string' || !request.computerId || request.computerId.length > 256) {
      throw new Error('Invalid screen computer.')
    }
    const target = resolveTarget(request.computerId)
    if (!target || target.computerId !== request.computerId || !Number.isInteger(target.vncPort) || target.vncPort < 1 || target.vncPort > 65535) {
      throw new Error("This computer's screen tunnel isn't open. Connect Over SSH and try again.")
    }
    const current = live.get(target.computerId)
    if (current && !current.window.isDestroyed() && current.vncPort === target.vncPort) {
      current.window.show()
      current.window.focus()
      return
    }
    if (pending.has(target.computerId)) return
    pending.add(target.computerId)
    let closeBridge: (() => void) | undefined
    try {
      closeComputerScreen(target.computerId)
      // Chromium file origins may serialize as file:// or as an opaque null origin.
      const origins = screenLocation.protocol === 'file:' ? ['file://', 'null'] : [screenLocation.origin]
      const bridge = await bridgeVnc(target.vncPort, origins)
      closeBridge = bridge.close
      if (resolveTarget(target.computerId)?.vncPort !== target.vncPort) throw new Error('The screen tunnel closed.')
      const window = new BrowserWindow({
        width: 1280, height: 800, minWidth: 640, minHeight: 400,
        title: target.name ? `Screen · ${target.name}` : 'Screen',
        backgroundColor: '#000000', autoHideMenuBar: true,
        webPreferences: {
          preload: join(__dirname, '../preload/index.js'), sandbox: true, contextIsolation: true,
          nodeIntegration: false, webSecurity: true, backgroundThrottling: false,
        },
      })
      const location = new URL(screenLocation)
      location.searchParams.set('computer', target.computerId)
      live.set(target.computerId, {
        vncPort: target.vncPort, view: { computerId: target.computerId, name: target.name, url: bridge.url },
        close: bridge.close, window,
      })
      window.webContents.setIgnoreMenuShortcuts(true)
      window.webContents.setWindowOpenHandler(() => ({ action: 'deny' }))
      window.webContents.on('will-navigate', (event, destination) => {
        if (destination !== location.href) event.preventDefault()
      })
      window.webContents.on('will-redirect', (event) => event.preventDefault())
      window.on('closed', () => {
        if (live.get(target.computerId)?.window === window) closeComputerScreen(target.computerId)
      })
      await window.loadURL(location.href)
    } catch (error) {
      closeBridge?.()
      closeComputerScreen(target.computerId)
      throw error
    } finally {
      pending.delete(target.computerId)
    }
  })

  ipcMain.handle('screen:target', (event, computerId: string) => {
    if (typeof computerId !== 'string') throw new Error('Invalid screen computer.')
    const current = live.get(computerId)
    const location = new URL(screenLocation)
    location.searchParams.set('computer', computerId)
    authorize(event, current?.window ?? null, location)
    if (!current || resolveTarget(computerId)?.vncPort !== current.vncPort) {
      closeComputerScreen(computerId)
      return null
    }
    return current.view
  })
}
