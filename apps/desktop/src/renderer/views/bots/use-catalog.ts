import { useEffect, useState } from 'react'
import type { BotCatalog } from '@shared/catalog'
import type { BotStore } from '../../store/bot-store'
import { errorText } from './drafts'

export interface CatalogState {
  catalog: BotCatalog | null
  failure: string | null
}

/** The selected computer's ready-made bots and teams; an older host without a catalog fails quietly to empty. */
export function useCatalog(store: BotStore): CatalogState {
  const [state, setState] = useState<CatalogState>({ catalog: null, failure: null })
  const online = store.connection.kind === 'online'
  useEffect(() => {
    if (!online) return
    let live = true
    store.botCatalog().then(
      (catalog) => live && setState({ catalog, failure: null }),
      (e: unknown) => live && setState({ catalog: null, failure: errorText(e) }),
    )
    return () => {
      live = false
    }
  }, [store, online])
  return state
}
