import type { Bot, Entry } from '@shared/models'
import type { BotStore } from '../../store/bot-store'

/**
 * A chat as one self-contained HTML file: the messages, cards and notices, plainly styled, with no
 * scripts and no link back to Codync. Meant to be read by someone else or kept as a record; what the
 * agent did in its turn (tools, thinking) stays out, as in the chat itself.
 */
const escape = (text: string) =>
  text.replace(/[&<>"']/g, (character) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[character] ?? character)

const roleOf = (entry: Entry, bot: Bot) =>
  entry.kind === 'user' ? 'You' : entry.kind === 'permission' ? 'Approval' : entry.kind === 'notice' ? 'Notice' : bot.name

const textOf = (entry: Entry) =>
  entry.kind === 'permission' ? String(entry.data.title ?? 'Approval') : String(entry.data.text ?? '')

const when = (ms: number) => new Date(ms).toLocaleString(undefined, { year: 'numeric', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })

export function transcriptHtml(bot: Bot, entries: Entry[]): string {
  const ordered = [...entries].sort((a, b) => a.createdAt - b.createdAt)
  const messages = ordered
    .filter((entry) => ['user', 'agent', 'permission', 'notice'].includes(entry.kind))
    .map((entry) => {
      const text = textOf(entry).trim()
      if (!text) return ''
      const role = roleOf(entry, bot)
      return `<article class="message ${escape(entry.kind)}">
  <header><span class="role">${escape(role)}</span><time>${escape(when(entry.createdAt))}</time></header>
  <div class="text">${escape(text)}</div>
</article>`
    })
    .filter(Boolean)
    .join('\n')
  const title = `${bot.name} — Codync chat`
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escape(title)}</title>
<style>
  :root { color-scheme: light dark; --text: #141414; --secondary: #6b6b6b; --line: #e6e6e6; --surface: #f6f6f6; }
  @media (prefers-color-scheme: dark) { :root { --text: #ededed; --secondary: #a0a0a0; --line: #2c2c2c; --surface: #1b1b1b; } }
  * { box-sizing: border-box; }
  body { margin: 0 auto; max-width: 760px; padding: 40px 22px 80px; background: var(--surface); color: var(--text);
    font: 15px/1.55 -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif; }
  h1 { font-size: 24px; margin: 0 0 4px; }
  .meta { color: var(--secondary); font-size: 12.5px; margin-bottom: 26px; }
  .message { background: Canvas; border: 1px solid var(--line); border-radius: 12px; padding: 12px 14px; margin: 0 0 10px; }
  .message.user { border-left: 3px solid var(--secondary); }
  .message.permission, .message.notice { border-left: 3px solid #b88700; }
  .message header { display: flex; gap: 10px; align-items: baseline; justify-content: space-between; margin-bottom: 6px; }
  .role { font-weight: 600; font-size: 12.5px; }
  time { color: var(--secondary); font-size: 11.5px; }
  .text { white-space: pre-wrap; overflow-wrap: anywhere; }
  @media print { body { background: white; max-width: none; } .message { break-inside: avoid; } }
</style>
</head>
<body>
<h1>${escape(bot.name)}</h1>
<div class="meta">Codync chat · exported ${escape(when(Date.now()))}${bot.model ? ` · ${escape(bot.model)}` : ''}</div>
${messages || '<p class="meta">This chat has no messages yet.</p>'}
</body>
</html>`
}

async function sha256(bytes: Uint8Array): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', bytes)
  return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('')
}

/**
 * Saves the chat as an HTML file (the native Save dialog) or opens it in the browser. Only the
 * messages already on this device are included: the same window the chat itself shows.
 */
export async function shareTranscript(store: BotStore, botId: string, mode: 'save' | 'open') {
  const bot = store.bots.get(botId)
  if (!bot) throw new Error('This chat is no longer open.')
  const html = transcriptHtml(bot, store.chatEntries(botId))
  const bytes = new TextEncoder().encode(html)
  const id = crypto.randomUUID()
  if (mode === 'open') {
    await window.codync.artifacts.openInBrowser(id, bytes)
    return
  }
  const name = `${bot.name.replace(/[/\\\x00-\x1f\x7f]/g, '-').slice(0, 150) || 'chat'} — chat.html`
  const handle = await window.codync.files.begin({ id, name, size: bytes.length, sha256: await sha256(bytes) })
  if (!handle) return
  try {
    const chunk = 512 * 1024
    for (let offset = 0; offset < bytes.length; offset += chunk) {
      await window.codync.files.write(handle, offset, bytes.subarray(offset, offset + chunk))
    }
    await window.codync.files.finish(handle)
  } finally {
    await window.codync.files.cancel(handle).catch(() => {})
  }
}
