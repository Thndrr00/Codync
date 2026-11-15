// Marketplace reputation from the Codync cloud: what other people rated an app, and how many
// accounts added it. Reading is public; rating and counting an install need the signed-in account.

export interface AppReputation {
  appId: string
  /** The average, 1 to 5; 0 until someone rates it. */
  stars: number
  /** How many people rated it. */
  count: number
  installs: number
}

type CloudAnswer = { ok: true; value: unknown } | { ok: false; error: { status: number; code: string; message: string | null } }

async function request(method: string, path: string, body: unknown, signed: boolean): Promise<unknown> {
  const answer: CloudAnswer = await window.codync.cloud.request(method, path, body, signed)
  if (!answer.ok) throw new Error(answer.error.message ?? answer.error.code)
  return answer.value
}

/** Every app with a rating or an install, by id; an empty map without a cloud or an account. */
export async function fetchReputation(): Promise<Map<string, AppReputation>> {
  try {
    const value = (await request('GET', 'v1/marketplace/reputation', null, false)) as { apps?: AppReputation[] }
    return new Map((value.apps ?? []).map((app) => [app.appId, app]))
  } catch {
    return new Map()
  }
}

/** Rates an app 1 to 5; rating again replaces this account's stars. */
export async function rateApp(appId: string, stars: number): Promise<void> {
  await request('PUT', `v1/marketplace/apps/${encodeURIComponent(appId)}/rating`, { stars }, true)
}

/** The account added the app to a computer; the cloud counts it once per account and app. */
export async function recordInstall(appId: string): Promise<void> {
  await request('POST', `v1/marketplace/apps/${encodeURIComponent(appId)}/install`, {}, true).catch(() => {})
}

export interface ChangelogEntry {
  appId: string
  version: string
  notes: string
  publishedAt: number
}

export interface MarketplaceMeta {
  /** The newest published entry per app. */
  changelog: Map<string, ChangelogEntry>
  /** Creators the maintainers vouched for. */
  publishers: Set<string>
}

/** What's new per app and the verified creators; empty when the cloud has neither yet. */
export async function fetchMarketplaceMeta(): Promise<MarketplaceMeta> {
  const [notes, publishers] = await Promise.allSettled([
    request('GET', 'v1/marketplace/changelog', null, false),
    request('GET', 'v1/marketplace/publishers', null, false),
  ])
  const changelog = new Map<string, ChangelogEntry>()
  if (notes.status === 'fulfilled') {
    for (const entry of ((notes.value as { apps?: ChangelogEntry[] }).apps ?? [])) {
      if (!changelog.has(entry.appId)) changelog.set(entry.appId, entry)
    }
  }
  const verified = new Set<string>(
    publishers.status === 'fulfilled' ? ((publishers.value as { publishers?: string[] }).publishers ?? []) : [],
  )
  return { changelog, publishers: verified }
}
