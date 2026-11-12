import { useEffect } from 'react'
import type { Entry, SharedFile } from '../../../shared/models'
import { useModel } from '../../lib/observable.ts'
import { islandThreadNavigation } from '../../store/island-thread-navigation.ts'

/** Resume an island destination after the target thread mounts, without fetching it twice. */
export function useIslandThreadNavigation(
  ref: string,
  onFile: (entry: Entry, file: SharedFile) => void,
  onArtifact: (id: string) => void,
) {
  const navigation = useModel(islandThreadNavigation)
  const sequence = navigation.pending?.sequence
  useEffect(() => {
    const destination = navigation.consume(ref)
    if (!destination) return
    if (destination.kind === 'file') onFile(destination.entry, destination.file)
    else onArtifact(destination.artifactId)
  }, [ref, sequence, navigation, onFile, onArtifact])
}
