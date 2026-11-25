//! A small desktop overlay that shows where a background agent moved or clicked (OpenBot
//! computer-use.md). It is a transparent, non-focusable, click-through window: the user's own pointer
//! and front app stay theirs. No new dependency and no new IPC channel: the page is a data URL drawn
//! with plain CSS, and the dot is moved with `executeJavaScript` on this window only.

import { BrowserWindow, screen } from 'electron'
import { cursor_point, type AgentCursorDisplay, type AgentCursorEvent } from '../renderer/lib/cursor-point'

/** The dot and its fade. Reduced motion is honoured by the media query, not by JS timing. */
const PAGE = `<!doctype html><html><head><meta charset="utf-8"><style>
  html,body{margin:0;width:100%;height:100%;background:transparent;overflow:hidden}
  #dot{position:absolute;width:12px;height:12px;border-radius:50%;border:2px solid #fff;
    background:#0a84ffaa;box-shadow:0 1px 4px #0009;opacity:0;transform:translate(-50%,-50%);
    transition:opacity 200ms ease;pointer-events:none}
  @media (prefers-reduced-motion: reduce){#dot{transition:none}}
</style></head><body><div id="dot"></div><script>
  let timer;
  window.mark = (point) => {
    const dot = document.getElementById('dot');
    dot.style.left = point.x + 'px';
    dot.style.top = point.y + 'px';
    dot.style.opacity = point.click ? '0.95' : '0.55';
    clearTimeout(timer);
    timer = setTimeout(() => { dot.style.opacity = '0'; }, point.click ? 900 : 450);
  };
</script></body></html>`

/** One overlay per app process, created lazily on the first point. */
export class AgentCursor {
  private window: BrowserWindow | null = null

  /** Marks one agent move/click on the display it belongs to. Invalid events are ignored. */
  mark(event: AgentCursorEvent): void {
    if (!Number.isFinite(event.x) || !Number.isFinite(event.y)) return
    const display = screen.getDisplayNearestPoint({ x: event.x, y: event.y })
    const bounds: AgentCursorDisplay = display.bounds
    const point = cursor_point(event, bounds)
    if (!point) return
    this.show(bounds)
    void this.window?.webContents.executeJavaScript(`window.mark(${JSON.stringify(point)})`).catch(() => {
      // A window that closed between the bounds read and the call simply misses this one dot.
    })
  }

  /** Places the overlay on the display, without taking focus or mouse events. */
  private show(display: AgentCursorDisplay): void {
    if (this.window && !this.window.isDestroyed()) {
      this.window.setBounds(display)
      return
    }
    const win = new BrowserWindow({
      ...display,
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
      show: false,
      webPreferences: { sandbox: true, contextIsolation: true, nodeIntegration: false },
    })
    win.setIgnoreMouseEvents(true)
    win.setAlwaysOnTop(true, process.platform === 'darwin' ? 'main-menu' : 'screen-saver')
    if (process.platform !== 'darwin') win.setVisibleOnAllWorkspaces(true)
    win.once('ready-to-show', () => win.showInactive())
    win.on('closed', () => {
      if (this.window === win) this.window = null
    })
    void win.loadURL(`data:text/html;charset=utf-8,${encodeURIComponent(PAGE)}`)
    this.window = win
  }

  close(): void {
    this.window?.close()
    this.window = null
  }
}

/** The shared overlay; a caller with an agent pointer event calls `mark` and nothing else. */
export const agentCursor = new AgentCursor()
