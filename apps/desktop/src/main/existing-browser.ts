/** A retained window only. This interface cannot create tabs, leases or browser sessions. */
export interface ExistingBrowserWindow {
  isDestroyed(): boolean
  show(): void
  focus(): void
  isMinimized?(): boolean
  restore?(): void
}

export function existingBrowser(window: ExistingBrowserWindow | null): window is ExistingBrowserWindow {
  return window !== null && !window.isDestroyed()
}

export function focusExistingBrowser(window: ExistingBrowserWindow | null): boolean {
  if (!existingBrowser(window)) return false
  if (window.isMinimized?.()) window.restore?.()
  window.show()
  window.focus()
  return true
}
