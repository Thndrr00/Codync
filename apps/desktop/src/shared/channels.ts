/** Messaging channels (Telegram, Slack) as the host reports them, and the wording the Channels screen uses. */

export interface TelegramBotStatus {
  name: string
  username: string
  botId: string
  botName: string
  allowed: number[]
  /** An SF Symbol shown as the bot's picture, or empty. */
  icon?: string
  /** What the bot knows about this channel (given before every message). */
  memory?: string
  /** `ok`, `starting`, or `error: …`. */
  state: string
  lastReceivedAt: number | null
  pairingExpiresAt: number | null
}

export interface SlackStatus {
  configured: boolean
  socketMode: boolean
  workspaceId: string | null
  bindings: { channelId: string; botId: string; allowedUsers: string[]; memory?: string; icon?: string }[] | null
}

/**
 * One messaging conversation linked to a Codync thread (card K13): the host's `conversationLinks` entry, which
 * `linkConversation` and `unlinkConversation` change.
 */
export interface ConversationLink {
  botId: string
  /** `telegram`, `slack`, `discord` or `matrix`. */
  source: string
  channel: string
  /** The provider's thread inside the channel: a Slack thread's timestamp, empty for a Telegram chat or Discord channel. */
  thread: string
  /** The Codync thread the conversation writes into. */
  rootId: string
  createdAt: number
  updatedAt: number
}

/** The Matrix connection, as `matrixStatus` reports it. Room ids look like `!opaque:server`. */
export interface MatrixBinding {
  roomId: string
  botId: string
  allowedUsers: string[]
  memory?: string
  icon?: string
}

export interface MatrixStatus {
  configured: boolean
  homeserver: string | null
  userId: string | null
  deviceId: string | null
  bindings: MatrixBinding[] | null
  /** `ok`, `starting`, or `error: …`. */
  state: string
  deliveries: { id: string; state: string }[]
}

/** One Signal chat bound to a bot: a phone number or `group:<id>`. */
export interface SignalBinding {
  chat: string
  botId: string
  allowedUsers: string[]
  memory?: string
  icon?: string
}

/** The signal-cli connection, as `signalStatus` reports it. */
export interface SignalStatus {
  configured: boolean
  number: string | null
  daemon: string | null
  bindings: SignalBinding[] | null
  /** `ok`, `starting`, `notRunning: ...` or `error: ...`. */
  state: string
  /** The exact signal-cli commands when the daemon is not set up. */
  setup: string | null
  deliveries: { id: string; state: string }[]
}

/** A Teams connection, as `teamsStatus` reports it. */
export interface TeamsStatus {
  configured: boolean
  appId: string | null
  tenantId: string | null
  botId: string | null
  managedIdentity: boolean
  allowedUsers: string[] | null
  allowedTenants: string[] | null
  memory?: string
  icon?: string
  /** The path the Azure Bot registration's messaging endpoint points at. */
  endpoint: string | null
  /** What the person must do in Azure; the host cannot create the registration. */
  registration: string
  state: string
  deliveries: { id: string; state: string }[]
}

/** A Signal number in E.164, or a `group:<id>` chat. */
export const signalChatOk = (chat: string) =>
  chat.length <= 512 && (/^\+[1-9]\d{6,14}$/.test(chat) || /^group:[A-Za-z0-9+/=_-]{8,512}$/.test(chat))
export const signalNumberOk = (number: string) => /^\+[1-9]\d{6,14}$/.test(number)
export const signalPersonOk = (person: string) =>
  signalNumberOk(person) || /^[0-9a-fA-F-]{32,64}$/.test(person)

export function signalStateText(state: string): string {
  if (state === 'ok') return 'Connected'
  if (state === 'starting') return 'Connecting...'
  if (state.startsWith('notRunning')) return 'signal-cli is not running'
  return 'Cannot reach signal-cli'
}

export function teamsStateText(state: string): string {
  if (state === 'ok') return 'Connected'
  if (state === 'starting') return 'Connecting...'
  return 'Not set up'
}
/** A homeserver address: https, or http to a server on this computer's own network. */
export const matrixHomeserverOk = (url: string) => url.length <= 300 && /^https?:\/\/[^\s/]+(\/[^\s]*)?$/.test(url)
/** A full Matrix user id: `@name:server`. */
export const matrixUserOk = (id: string) => id.length <= 255 && /^@[^:\s]+:[^\s]+$/.test(id)
/** A room id or alias: `!opaque:server` or `#alias:server`. */
export const matrixRoomOk = (id: string) => id.length <= 255 && /^[!#][^:\s]+:[^\s]+$/.test(id)

export function matrixStateText(state: string): string {
  if (state === 'ok') return 'Connected'
  if (state === 'starting') return 'Connecting…'
  return 'Cannot reach Matrix'
}

/** The Discord connection, as `discordStatus` reports it. Snowflakes are digits. */
export interface DiscordStatus {
  configured: boolean
  guildId: string | null
  botUserId: string | null
  bindings: { channelId: string; botId: string; allowedUsers: string[]; memory?: string; icon?: string }[] | null
  /** `ok`, `starting`, or `error: …`. */
  state: string
  deliveries: { id: string; state: string }[]
}

export const discordIdOk = (id: string) => /^\d{1,32}$/.test(id)
export const discordTokenOk = (token: string) =>
  token.length <= 200 && /^[A-Za-z0-9_-]{20,}\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]{20,}$/.test(token)

export function discordStateText(state: string): string {
  if (state === 'ok') return 'Connected'
  if (state === 'starting') return 'Connecting…'
  return 'Cannot reach Discord'
}

/** Same shape rules as the host's, so a mistake is told before the call. */
export const telegramNameOk = (name: string) => /^[a-z0-9-]{1,32}$/.test(name)
export const telegramTokenOk = (token: string) => token.length <= 128 && /^\d+:[A-Za-z0-9_-]{30,}$/.test(token)

export function telegramStateText(state: string): string {
  if (state === 'ok') return 'Connected'
  if (state === 'starting') return 'Starting…'
  return 'Cannot reach Telegram'
}

export function lastReceivedText(at: number | null, now: number): string {
  if (at === null) return 'Nothing received yet'
  const minutes = Math.max(0, Math.floor((now - at) / 60_000))
  if (minutes < 1) return 'Last message just now'
  if (minutes < 60) return `Last message ${minutes} min ago`
  if (minutes < 60 * 24) return `Last message ${Math.floor(minutes / 60)} h ago`
  return `Last message ${Math.floor(minutes / (60 * 24))} d ago`
}

/** What the person does with a pairing code. */
export function pairingInstruction(code: string, username: string): string {
  const where = username ? `@${username}` : 'your Telegram bot'
  return `Send /start ${code} to ${where} in Telegram. The code works once, for 10 minutes.`
}

export const allowedText = (count: number) => (count === 1 ? '1 person allowed' : `${count} people allowed`)
