/** The computer's ceiling on local mini-apps (`localApps`): a safeguard on that computer, not a plan. */

export interface LocalApps {
  /** Every registered app the computer holds, from every bot. */
  count: number
  /** The saved ceiling; a new app past it is refused. */
  limit: number
  /** The ceilings the computer offers. */
  choices: number[]
  /** How many bots made the apps it holds. */
  botsWithApps: number
}
