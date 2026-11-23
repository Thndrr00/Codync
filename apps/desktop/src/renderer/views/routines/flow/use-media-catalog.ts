import { useEffect, useState } from 'react'
import type { FlowMediaCatalog } from '../../../lib/routine-flow.ts'
import { useStore } from '../../../store/context'

/**
 * The owner bot's live media catalog, read once for the media composer and inspector. The host's
 * availability flag and price are optional fields on this answer: an older host leaves them out,
 * so the picker stays unfiltered and the buttons say Run/Retry without a price.
 */
export function useFlowMediaCatalog(botId: string): { answer: FlowMediaCatalog | null; failed: boolean } {
  const store = useStore()
  const [answer, setAnswer] = useState<FlowMediaCatalog | null>(null)
  const [failed, setFailed] = useState(false)
  useEffect(() => {
    let alive = true
    setAnswer(null)
    setFailed(false)
    void store.mediaModels(botId)
      .then((models) => { if (alive) setAnswer(models) })
      .catch(() => { if (alive) setFailed(true) })
    return () => { alive = false }
  }, [botId, store])
  return { answer, failed }
}
