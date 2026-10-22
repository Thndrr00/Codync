// Windows Jump List: the taskbar icon's right-click menu lists the pinned bots, as the Android launcher
// shortcuts do. A click starts the app (or its running instance) with `--open-bot=<computerId>/<botId>`.

export const OPEN_BOT_FLAG = '--open-bot='
export const JUMP_LIST_LIMIT = 4

interface ListedBot {
  ref: string
  name: string
  pinned: boolean
}

/** The bots the Jump List offers: a few pinned ones, in roster order. */
export function jumpListBots<T extends ListedBot>(bots: T[], limit = JUMP_LIST_LIMIT): T[] {
  return bots.filter((bot) => bot.pinned && bot.name.trim()).slice(0, Math.max(0, limit))
}

const REF = /^[A-Za-z0-9_-]{1,64}\/[^\s/\u0000-\u001f]{1,128}$/

/** The argument a Jump List entry starts the app with. */
export const openBotArgument = (ref: string) => `${OPEN_BOT_FLAG}${ref}`

/** The bot a command line asks to open; anything that isn't a plain `computer/bot` pair is ignored. */
export function botRefFromArgv(argv: string[]): string | null {
  const arg = argv.find((a) => a.startsWith(OPEN_BOT_FLAG))
  const ref = arg?.slice(OPEN_BOT_FLAG.length)
  return ref && REF.test(ref) ? ref : null
}
