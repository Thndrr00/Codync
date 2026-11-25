/**
 * The launch splash stays up at least this long so a fast renderer does not flash a half-built
 * workspace (OpenBot's splash-min-shown). A slow one becomes usable the moment it is ready.
 */
export const SPLASH_MIN_MS = 1500

/** True while the splash surface is still covering the workspace. */
export function splashVisible(elapsedMs: number, ready: boolean, minimumMs = SPLASH_MIN_MS): boolean {
  if (!Number.isFinite(elapsedMs) || !ready) return true
  return Math.max(0, elapsedMs) < minimumMs
}

/** The inverse: the workspace shows as soon as it is ready and the splash has been seen. */
export function workspaceVisible(elapsedMs: number, ready: boolean, minimumMs = SPLASH_MIN_MS): boolean {
  return !splashVisible(elapsedMs, ready, minimumMs)
}

/** How long to wait before showing the workspace; zero when it is already ready. */
export function splashDelay(elapsedMs: number, ready: boolean, minimumMs = SPLASH_MIN_MS): number {
  if (!Number.isFinite(elapsedMs) || !ready) return minimumMs
  return Math.max(0, minimumMs - elapsedMs)
}
