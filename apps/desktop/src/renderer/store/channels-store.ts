import type { HostClient } from '../client/host-client'
import { Observable } from '../lib/observable.ts'
import type { ConversationLink, DiscordStatus, MatrixStatus, SlackStatus, TelegramBotStatus } from '../../shared/channels'

interface TelegramReply { bots: TelegramBotStatus[]; result?: { code?: string; expiresAt?: number } }

/** The messaging channels of this computer's host: Telegram bots (add, pair, allow) and the Slack connection. */
export class ChannelsStore extends Observable {
  telegram: TelegramBotStatus[] = []
  slack: SlackStatus | null = null
  discord: DiscordStatus | null = null
  matrix: MatrixStatus | null = null
  loading = false
  busy = false
  error: string | null = null
  /** The pairing code just made, shown once. */
  pairing: { name: string; code: string } | null = null
  private active = true
  private readonly client: () => Promise<Pick<HostClient, 'call'>>
  constructor(client: () => Promise<Pick<HostClient, 'call'>>) { super(); this.client = client }
  resume() { this.active = true }
  dispose() { this.active = false; this.loading = false; this.busy = false }

  async load() {
    if (!this.active || this.loading) return
    this.loading = true; this.error = null; this.changed()
    try {
      const host = await this.client()
      const [telegram, slack, discord, matrix] = await Promise.all([
        host.call<TelegramReply>('telegramStatus', {}),
        host.call<SlackStatus>('slackStatus', {}),
        host.call<DiscordStatus>('discordStatus', {}),
        host.call<MatrixStatus>('matrixStatus', {}),
      ])
      if (!this.active) return
      this.telegram = telegram.bots
      this.slack = slack
      this.discord = discord
      this.matrix = matrix
    } catch (error) { if (this.active) this.error = String(error) }
    finally { if (this.active) { this.loading = false; this.changed() } }
  }

  /** Runs one change and keeps the answer the host gives. */
  private async act(method: string, body: Record<string, unknown>): Promise<TelegramReply | null> {
    if (!this.active || this.busy) return null
    this.busy = true; this.error = null; this.changed()
    try {
      const reply = await (await this.client()).call<TelegramReply>(method, body)
      if (this.active) this.telegram = reply.bots
      return reply
    } catch (error) { if (this.active) this.error = String(error); return null }
    finally { if (this.active) { this.busy = false; this.changed() } }
  }

  async addTelegram(name: string, token: string, botId: string) {
    return (await this.act('addTelegramBot', { name, token, botId })) !== null
  }
  async removeTelegram(name: string) {
    if (await this.act('removeTelegramBot', { name }) && this.pairing?.name === name) { this.pairing = null; this.changed() }
  }
  async pair(name: string) {
    const reply = await this.act('pairTelegram', { name })
    const code = reply?.result?.code
    if (code) { this.pairing = { name, code }; this.changed() }
  }
  async setAllowed(name: string, userId: number, allowed: boolean) {
    await this.act('setTelegramAllowed', { name, userId, allowed })
  }

  /** Saves what the bot should know about a channel (a Telegram bot's name, or a Slack or Discord channel). */
  async setMemory(provider: 'slack' | 'discord' | 'telegram' | 'matrix', channel: string, memory: string) {
    if (!this.active || this.busy) return false
    this.busy = true; this.error = null; this.changed()
    try {
      await (await this.client()).call('setChannelMemory', { provider, channel, memory })
      return true
    } catch (error) { if (this.active) this.error = String(error); return false }
    finally { if (this.active) { this.busy = false; this.changed() } }
  }

  /** Gives a channel a picture (an SF Symbol name), or clears it. */
  async setIcon(provider: 'slack' | 'discord' | 'telegram' | 'matrix', channel: string, icon: string) {
    if (!this.active || this.busy) return false
    this.busy = true; this.error = null; this.changed()
    try {
      await (await this.client()).call('setChannelIcon', { provider, channel, icon })
      return true
    } catch (error) { if (this.active) this.error = String(error); return false }
    finally { if (this.active) { this.busy = false; this.changed() } }
  }

  /** Small facts this channel keeps, given to the bot with its memory. */
  async notes(provider: 'slack' | 'discord' | 'telegram' | 'matrix', channel: string): Promise<string[]> {
    const answer = await (await this.client()).call<{ notes: string[] }>('channelNotes', { provider, channel })
    return answer.notes ?? []
  }

  /** Adds one note; the host checks its length and the channel's count. */
  async addNote(provider: 'slack' | 'discord' | 'telegram' | 'matrix', channel: string, text: string): Promise<string[]> {
    const answer = await (await this.client()).call<{ notes: string[] }>('addChannelNote', { provider, channel, text })
    return answer.notes ?? []
  }

  /** Removes the note at `index`, as the channel's list has it. */
  async removeNote(provider: 'slack' | 'discord' | 'telegram' | 'matrix', channel: string, index: number): Promise<string[]> {
    const answer = await (await this.client()).call<{ notes: string[] }>('removeChannelNote', { provider, channel, index })
    return answer.notes ?? []
  }

  /** The conversations of this bot linked to a Codync thread (card K13). */
  async conversationLinks(botId: string): Promise<ConversationLink[]> {
    const answer = await (await this.client()).call<{ links: ConversationLink[] }>('conversationLinks', { botId })
    return answer.links ?? []
  }

  /** The chat's threads, for choosing the one a channel conversation shares. */
  async threads(botId: string): Promise<{ id: string; name: string }[]> {
    const answer = await (await this.client()).call<{ threads: { id: string; title: string; branchText: string | null }[] }>('threads', { botId })
    return (answer.threads ?? []).map((thread) => ({
      id: thread.id,
      name: thread.title.trim() || thread.branchText?.slice(0, 60).replaceAll('\n', ' ') || 'Thread',
    }))
  }

  /** Links a channel conversation to a thread; every link of the bot comes back. */
  async linkConversation(body: { botId: string; provider: string; channel: string; thread: string; threadId: string }): Promise<ConversationLink[]> {
    const answer = await (await this.client()).call<{ links: ConversationLink[] }>('linkConversation', body)
    return answer.links ?? []
  }

  /** Takes a channel conversation's link away; its next message starts over. */
  async unlinkConversation(body: { botId: string; provider: string; channel: string; thread: string }): Promise<ConversationLink[]> {
    const answer = await (await this.client()).call<{ links: ConversationLink[] }>('unlinkConversation', body)
    return answer.links ?? []
  }

  async disconnectSlack() {
    if (!this.active || this.busy) return
    this.busy = true; this.error = null; this.changed()
    try {
      this.slack = await (await this.client()).call<SlackStatus>('disconnectSlack', {})
    } catch (error) { if (this.active) this.error = String(error) }
    finally { if (this.active) { this.busy = false; this.changed() } }
  }

  /** Saves the Discord connection; the host checks the token and every channel first. */
  async setDiscord(body: {
    guildId: string
    botUserId: string
    botToken: string
    bindings: { channelId: string; botId: string; allowedUsers: string[] }[]
  }) {
    if (!this.active || this.busy) return false
    this.busy = true; this.error = null; this.changed()
    try {
      this.discord = await (await this.client()).call<DiscordStatus>('setDiscordChannel', body)
      return true
    } catch (error) { if (this.active) this.error = String(error); return false }
    finally { if (this.active) { this.busy = false; this.changed() } }
  }

  async disconnectDiscord() {
    if (!this.active || this.busy) return
    this.busy = true; this.error = null; this.changed()
    try {
      this.discord = await (await this.client()).call<DiscordStatus>('disconnectDiscord', {})
    } catch (error) { if (this.active) this.error = String(error) }
    finally { if (this.active) { this.busy = false; this.changed() } }
  }

  /** Saves the Matrix connection; the host signs in and checks every room first. */
  async setMatrix(body: {
    homeserver: string
    user: string
    accessToken: string
    password: string
    bindings: { roomId: string; botId: string; allowedUsers: string[] }[]
  }) {
    if (!this.active || this.busy) return false
    this.busy = true; this.error = null; this.changed()
    try {
      this.matrix = await (await this.client()).call<MatrixStatus>('setMatrixChannel', body, 60_000)
      return true
    } catch (error) { if (this.active) this.error = String(error); return false }
    finally { if (this.active) { this.busy = false; this.changed() } }
  }

  async disconnectMatrix() {
    if (!this.active || this.busy) return
    this.busy = true; this.error = null; this.changed()
    try {
      this.matrix = await (await this.client()).call<MatrixStatus>('disconnectMatrix', {})
    } catch (error) { if (this.active) this.error = String(error) }
    finally { if (this.active) { this.busy = false; this.changed() } }
  }
}
