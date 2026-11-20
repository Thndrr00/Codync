import type { Artifact, LibraryItem } from '@shared/artifacts'
import type { Entry, EntryData } from '@shared/models'
import type { LibraryFolder } from '../lib/library-folders.ts'
import type { SeedBot, SeedEntry } from './story.ts'

// The extra "Load sample data" story. Pure data plus two digests: it never reaches the network, the
// disk or the real host. The in-memory FakeHost adopts it when the action is picked in demo mode.

/** A side-thread reply carries the root entry's id; a root carries a `thread` summary. */
export type SampleEntry = SeedEntry & { threadId?: string }

export interface SampleArtifact {
  meta: Artifact
  html: string
}

export interface SampleData {
  bots: SeedBot[]
  entries: SampleEntry[]
  artifacts: SampleArtifact[]
  items: LibraryItem[]
  folders: LibraryFolder[]
}

const draftHTML = `<!doctype html><html><head><meta charset="utf-8"><title>Pace weekly totals</title>
<style>body{font:15px/1.6 system-ui,sans-serif;max-width:640px;margin:48px auto;color:#141414}
h1{font-size:24px}code{background:#f4f4f4;padding:1px 4px;border-radius:4px}</style></head><body>
<h1>Weekly totals are here</h1><p>Pace now groups runs by ISO week and sums distance and time. The keys look like
<code>2026-W53</code>, and the New Year rollover uses the Thursday of the week.</p>
<h2>What ships</h2><ul><li>Weekly distance and time in the dashboard</li><li>A guard against invalid dates</li>
<li>A test for the December-to-January rollover</li></ul></body></html>`

const referenceHTML = `<!doctype html><html><head><meta charset="utf-8"><title>Pace API</title>
<style>body{font:15px/1.6 system-ui,sans-serif;max-width:640px;margin:48px auto;color:#141414}
code{background:#f4f4f4;padding:1px 4px;border-radius:4px}</style></head><body>
<h1>Pace API</h1><p><code>weeklyTotals(runs)</code> takes runs shaped like
<code>{ date, distance, time }</code> and returns an object keyed by ISO week.</p>
<p>Weeks are grouped in UTC. An invalid date raises a <code>RangeError</code>.</p></body></html>`

/** The same hex digest `shared/files.ts` checks, so a read artifact passes its integrity check. */
async function digestHex(text: string) {
  const bytes = new TextEncoder().encode(text)
  const hash = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))
  return [...hash].map((b) => b.toString(16).padStart(2, '0')).join('')
}

const user = (id: string, botId: string, at: number, text: string, extra: EntryData = {}): SampleEntry => ({
  id, botId, kind: 'user', at, data: { text, status: 'sent', ...extra },
})
const reply = (id: string, botId: string, author: string, at: number, text: string, extra: EntryData = {}): SampleEntry => ({
  id, botId, kind: 'agent', at, data: { text, final: true, author, ...extra },
})

/** Builds the sample story once, with real digests for the two HTML artifacts. */
export async function buildSampleData(now = Date.now()): Promise<SampleData> {
  const [draftSha, referenceSha] = await Promise.all([digestHex(draftHTML), digestHex(referenceHTML)])
  const draftBytes = new TextEncoder().encode(draftHTML).length
  const referenceBytes = new TextEncoder().encode(referenceHTML).length

  const draft: Artifact = {
    id: 'sample-weekly-note', botId: 'scribe', title: 'Weekly totals — release note',
    size: draftBytes, sha256: draftSha, createdAt: now - 30 * 60_000, updatedAt: now - 28 * 60_000,
    entryId: 'sa-card',
  }
  const reference: Artifact = {
    id: 'sample-api-reference', botId: 'scribe', title: 'Pace API reference',
    size: referenceBytes, sha256: referenceSha, createdAt: now - 90 * 60_000, updatedAt: now - 85 * 60_000,
  }

  return {
    bots: [
      { id: 'scribe', name: 'Scribe', description: 'Turns changes into release notes', avatarColor: 'orange', avatarShape: 'squircle', backend: 'claude', model: 'sonnet', unread: 1 },
      { id: 'atlas', name: 'Atlas', description: 'Keeps the docs in sync with the code', avatarColor: 'violet', avatarShape: 'tablet', backend: 'codex', model: null, unread: 1 },
      { id: 'quill', name: 'Quill', description: 'Drafts the changelog and the team update', avatarColor: 'magenta', avatarShape: 'cloud', backend: 'claude', model: 'opus' },
    ],
    entries: [
      // Scribe publishes an artifact as a card in its chat.
      user('sa1', 'scribe', 34, 'Draft a short release note for the weekly totals change.'),
      reply('sa2', 'scribe', 'scribe', 33, 'Here is a first draft. I kept it to what people will notice.'),
      {
        id: 'sa-card', botId: 'scribe', kind: 'agent', at: 28,
        data: { text: 'Published the release note as an artifact.', final: true, author: 'scribe', artifact: { id: draft.id, title: draft.title, updatedAt: draft.updatedAt } },
      },
      user('sa4', 'scribe', 20, 'Make the rollover sentence clearer.'),
      reply('sa5', 'scribe', 'scribe', 19, 'Done — the note now names the Thursday rule in one line.'),

      // Atlas and Quill have a short, realistic exchange each.
      user('at1', 'atlas', 50, 'Update the API docs for weeklyTotals.'),
      reply('at2', 'atlas', 'atlas', 48, 'Added weeklyTotals(runs) and its UTC and invalid-date notes to the reference.'),
      user('qu1', 'quill', 15, 'Write the team update for this week.'),
      reply('qu2', 'quill', 'quill', 14, 'This week: weekly totals shipped, the New Year bug is fixed, and staging is next.'),

      // A thread on Scout's earlier message, with two replies from other bots.
      {
        id: 'sample-thread-root', botId: 'scout', kind: 'user', at: 12,
        data: {
          text: 'Should the weekly keys follow local time instead of UTC?', status: 'sent',
          thread: { count: 2, unread: 0, lastAt: now - 8 * 60_000, authors: ['scout', 'reviewer'] },
        },
      },
      {
        id: 'sample-thread-r1', botId: 'scout', kind: 'agent', at: 11, threadId: 'sample-thread-root',
        data: { text: 'UTC keeps the keys stable when a run is recorded on another device.', final: true, author: 'scout' },
      },
      {
        id: 'sample-thread-r2', botId: 'scout', kind: 'agent', at: 8, threadId: 'sample-thread-root',
        data: { text: 'Agreed — keep UTC and note it above the helper.', final: true, author: 'reviewer' },
      },
    ],
    artifacts: [{ meta: draft, html: draftHTML }, { meta: reference, html: referenceHTML }],
    items: [
      { kind: 'artifact', at: draft.updatedAt, folderId: 'sample-drafts', artifact: draft },
      { kind: 'artifact', at: reference.updatedAt, folderId: 'sample-published', artifact: reference },
    ],
    folders: [
      { id: 'sample-drafts', name: 'Drafts', at: draft.updatedAt, count: 1, items: ['artifact:sample-weekly-note'] },
      { id: 'sample-published', name: 'Published', at: reference.updatedAt, count: 1, items: ['artifact:sample-api-reference'] },
    ],
  }
}

/** The sample entries as full host entries, for the FakeHost's event stream. */
export function sampleEntries(data: SampleData, now: number): Omit<Entry, 'seq' | 'rev'>[] {
  return data.entries.map((entry) => {
    const at = now - entry.at * 60_000
    return {
      id: entry.id, botId: entry.botId, kind: entry.kind, turn: 1,
      createdAt: at, updatedAt: at, data: entry.data,
      ...(entry.threadId ? { threadId: entry.threadId } : {}),
    }
  })
}
