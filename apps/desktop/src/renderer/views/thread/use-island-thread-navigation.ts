import { useEffect, useState } from 'react'
import type { Entry, SharedFile } from '../../../shared/models'
import type { BotApp } from '../../../shared/bdots-models'
import { useModel } from '../../lib/observable.ts'
import { islandThreadNavigation, type IslandThreadDestination } from '../../store/island-thread-navigation.ts'

/** Resume an island destination after the target thread mounts, without fetching it twice. */
export function useIslandThreadNavigation(
  ref: string,
  onFile: (entry: Entry, file: SharedFile) => void,
  onArtifact: (id: string) => void,
  onMemory?: () => void,
  onOperation?: (panel: 'trace' | 'computer' | 'documents', documentId?: string) => void,
  onApp?: (app: BotApp) => void,
  owner?: object,
) {
  const [budget, setBudget] = useState(false)
  const [terminal, setTerminal] = useState<Extract<IslandThreadDestination, { kind: 'terminal' }> | null>(null)
  const navigation = useModel(islandThreadNavigation)
  const sequence = navigation.pending?.sequence
  useEffect(() => {
    const destination = navigation.consume(ref, owner)
    if (!destination) return
    if (destination.kind === 'file') onFile(destination.entry, destination.file)
    else if (destination.kind === 'artifact') onArtifact(destination.artifactId)
    else if (destination.kind === 'memory') onMemory?.()
    else if (destination.kind === 'terminal') setTerminal(destination)
    else if (destination.kind === 'budget') setBudget(true)
    else if (destination.kind === 'operationPanel') onOperation?.(destination.panel, destination.documentId)
    else if (destination.kind === 'miniApp') onApp?.(destination.app)
  }, [ref, sequence, navigation, onFile, onArtifact, onMemory, onOperation, onApp, owner])
  return { terminal: terminal?.ref === ref ? terminal : null, close: () => setTerminal(null), budget, closeBudget: () => setBudget(false) }
}
