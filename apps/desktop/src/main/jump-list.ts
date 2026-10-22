import { app } from 'electron'
import type { TraySummary, WindowCommand } from '../shared/ipc'
import { botRefFromArgv, jumpListBots, openBotArgument } from '../shared/jump-list'

type Bots = TraySummary['bots']

/**
 * The taskbar Jump List (Windows only): the pinned bots, each starting Codync on that conversation. A request
 * for a bot waits until the renderer reports a roster that has it, so a cold start or a bot that's gone never
 * opens another conversation.
 */
export class JumpList {
  private known = new Set<string>()
  private pending: string | null = null
  private requestedAt = 0
  /** What the list was last built from: a roster update that changes nothing must not rewrite it. */
  private shown = ''

  constructor(private send: (command: WindowCommand) => void) {}

  /** Called with every roster the renderer reports; refreshes the list and delivers a waiting request. */
  update(bots: Bots) {
    this.known = new Set(bots.map((b) => b.ref))
    this.flush()
    if (process.platform !== 'win32') return
    const listed = jumpListBots(bots)
    const shown = listed.map((bot) => `${bot.ref}\u0000${bot.name}`).join('\u0001')
    if (shown === this.shown) return
    this.shown = shown
    const items = listed.map((bot) => ({
      type: 'task' as const,
      title: bot.name.slice(0, 60),
      description: `Open ${bot.name}`,
      program: process.execPath,
      // A development run starts Electron with the app's path first; it may contain spaces, so quote it.
      args: `${process.defaultApp && process.argv[1] ? `"${process.argv[1]}" ` : ''}${openBotArgument(bot.ref)}`,
      iconPath: process.execPath,
      iconIndex: 0,
    }))
    try { app.setJumpList(items.length ? [{ type: 'custom', name: 'Pinned bots', items }] : null) }
    catch (error) { console.error('Jump List unavailable', error) }
  }

  /** A launch or second instance asked for a bot (from the command line). */
  request(argv: string[]) {
    const ref = botRefFromArgv(argv)
    if (!ref) return false
    this.pending = ref
    this.requestedAt = Date.now()
    this.flush()
    return true
  }

  /** Sign-out or reset: nothing of this account stays in the taskbar menu. */
  clear() {
    this.known.clear(); this.pending = null; this.shown = ''
    if (process.platform === 'win32') app.setJumpList(null)
  }

  private flush() {
    // A request nobody could serve in 30 s is dropped, not served by a later roster.
    if (this.pending && Date.now() - this.requestedAt > 30_000) this.pending = null
    if (!this.pending || !this.known.has(this.pending)) return
    const ref = this.pending
    this.pending = null
    this.send({ kind: 'openBot', ref })
  }
}
