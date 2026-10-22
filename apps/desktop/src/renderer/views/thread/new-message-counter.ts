import { useEffect, useRef, useState } from 'react'

/**
 * The "N new" on Jump to latest (idea 31) for one conversation (main chat or thread; make one
 * counter per scope). It counts distinct message ids that appear after the newest one the reader
 * had when they left the bottom, so a streaming edit (same id) and an older-history load (ids
 * before that message) never add to it. Back at the bottom it resets.
 */
export class NewMessageCounter {
  private anchor: string | null = null
  private known = new Set<string>()
  private fresh = new Set<string>()

  get count() {
    return this.fresh.size
  }

  update(ids: string[], atBottom: boolean, own: Set<string> = new Set()): number {
    if (atBottom || ids.length === 0) {
      this.reset()
      return 0
    }
    if (this.anchor === null) {
      this.anchor = ids[ids.length - 1] ?? null
      for (const id of ids) this.known.add(id)
      return 0
    }
    const anchorIndex = ids.lastIndexOf(this.anchor)
    // Entries after the anchor are new; if the anchor itself is gone, only ids never seen count.
    const candidates = anchorIndex >= 0 ? ids.slice(anchorIndex + 1) : ids
    for (const id of candidates) if (!this.known.has(id) && !own.has(id)) this.fresh.add(id)
    for (const id of ids) this.known.add(id)
    return this.fresh.size
  }

  reset() {
    this.anchor = null
    this.known.clear()
    this.fresh.clear()
  }
}

/** Feeds the counter with the conversation's message ids and whether the reader is on the newest. */
export function useNewMessageCount(ids: string[], following: boolean): number {
  const counter = useRef<NewMessageCounter | null>(null)
  if (counter.current === null) counter.current = new NewMessageCounter()
  const [count, setCount] = useState(0)
  const key = ids.join('\u0000')
  useEffect(() => {
    setCount(counter.current!.update(ids, following))
  }, [key, following])
  return count
}
