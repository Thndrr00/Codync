import type { BotStore } from '../../store/bot-store'

const CHUNK = 512 * 1024

const escape = (text: string) =>
  text.replace(/[&<>"']/g, (character) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[character] ?? character)

/** A plain, self-contained page for a document: its title and Markdown body as preformatted text. */
export function documentHtml(title: string, body: string): string {
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escape(title)}</title>
<style>
  :root { color-scheme: light dark; --text: #141414; --secondary: #6b6b6b; --surface: #f6f6f6; }
  @media (prefers-color-scheme: dark) { :root { --text: #ededed; --secondary: #a0a0a0; --surface: #1b1b1b; } }
  body { margin: 0 auto; max-width: 760px; padding: 48px 24px 120px; background: var(--surface); color: var(--text);
    font: 15px/1.6 -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif; }
  h1 { font-size: 28px; margin: 0 0 6px; }
  .meta { color: var(--secondary); font-size: 12.5px; margin-bottom: 28px; }
  pre { white-space: pre-wrap; overflow-wrap: anywhere; font: 14px/1.55 ui-monospace, 'SF Mono', Consolas, monospace; }
  @media print { body { background: white; max-width: none; } }
</style>
</head>
<body>
<h1>${escape(title)}</h1>
<div class="meta">Shared from Codync · ${escape(new Date().toLocaleString())}</div>
<pre>${escape(body)}</pre>
</body>
</html>`
}

/** Stores a page on the computer and puts its read-only link on the clipboard. */
export async function publishPage(store: BotStore, title: string, html: string): Promise<string> {
  const { url } = await (await store.ready()).call<{ url: string }>('publishShare', { title, html })
  await window.codync.app.copy(url)
  return url
}

async function sha256(bytes: Uint8Array): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', bytes)
  return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('')
}

/** Saves text as a file with the native Save dialog. */
export async function saveTextFile(name: string, text: string): Promise<void> {
  const bytes = new TextEncoder().encode(text)
  const id = crypto.randomUUID()
  const handle = await window.codync.files.begin({ id, name, size: bytes.length, sha256: await sha256(bytes) })
  if (!handle) return
  try {
    for (let offset = 0; offset < bytes.length; offset += CHUNK) {
      await window.codync.files.write(handle, offset, bytes.subarray(offset, offset + CHUNK))
    }
    await window.codync.files.finish(handle)
  } finally {
    await window.codync.files.cancel(handle).catch(() => {})
  }
}
