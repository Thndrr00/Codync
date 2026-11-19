import { useCallback, useEffect, useRef, useState } from 'react'
import type { BotStore } from './bot-store'
import { useStore } from './context'

/**
 * What a bot's documentation index holds (`docsStatus`): the folder the host walked, the files it
 * found there, and whether a limit cut the walk short.
 */
export interface DocsIndexStatus {
  folder: string
  files: number
  bytes: number
  truncated: boolean
  builtAt: number
}

/** The first read walks the folder on the computer, which can take a while on a large repository. */
const TIMEOUT_MS = 60_000

async function read(store: BotStore, method: 'docsStatus' | 'setDocsFolder', body: { botId: string; path?: string }) {
  return (await store.ready()).call<DocsIndexStatus>(method, body, TIMEOUT_MS)
}

function reason(error: unknown, fallback: string) {
  return error instanceof Error && error.message ? error.message : fallback
}

/**
 * One bot's documentation and code index on the computer (the `codync-docs` server): reads what the
 * host indexed and names the folder it reads. The host builds, caches and limits the index; this only
 * shows it and points it elsewhere.
 */
export function useDocsIndex(botId: string) {
  const store = useStore()
  const [status, setStatus] = useState<DocsIndexStatus | null>(null)
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  /** Bumped by `reload`: the read below runs again. */
  const [attempt, setAttempt] = useState(0)
  /** A read and a change can overlap: only the newest answer is shown. */
  const ticket = useRef(0)
  const online = store.connection.kind === 'online'

  // The folder the host reads, every time the settings open (another session may have moved it).
  useEffect(() => {
    if (!online) {
      setStatus(null)
      setLoading(false)
      return
    }
    const asked = ++ticket.current
    setLoading(true)
    setError(null)
    void (async () => {
      try {
        const value = await read(store, 'docsStatus', { botId })
        if (asked !== ticket.current) return
        setStatus(value)
        setError(null)
      } catch (e) {
        if (asked === ticket.current) setError(reason(e, "Couldn't read the documentation index."))
      } finally {
        if (asked === ticket.current) setLoading(false)
      }
    })()
    return () => {
      ticket.current++
    }
  }, [attempt, botId, online, store])

  /** Reads the folder again: a load that failed, or one to refresh after the host moved the index. */
  const reload = useCallback(() => setAttempt((n) => n + 1), [])

  /** Names the folder to index, or with an empty path the bot's working folder, and shows what the host indexed. */
  const choose = useCallback(
    (path: string) => {
      if (!online || busy) return
      const asked = ++ticket.current
      setBusy(true)
      setError(null)
      void (async () => {
        try {
          const value = await read(store, 'setDocsFolder', { botId, path })
          if (asked !== ticket.current) return
          setStatus(value)
          setError(null)
        } catch (e) {
          if (asked === ticket.current) setError(reason(e, "Couldn't change the indexed folder."))
        } finally {
          // The spinner follows this request alone: a stale answer may not leave the buttons disabled.
          setBusy(false)
        }
      })()
    },
    [botId, busy, online, store],
  )

  return { status, loading, busy, error, choose, reload }
}
