import { Button, IconButton } from '../../components/Controls'
import { HStack, Spacer } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'

/** The actions for a multi-item selection, shown under the Library's files while any are selected. */
export function LibrarySelectionBar({ count, busy, onDownload, onDelete, onClear }: {
  count: number
  busy: boolean
  onDownload: () => void
  onDelete: () => void
  onClear: () => void
}) {
  const tr = useTr()
  if (count === 0) return null
  return (
    <HStack gap={8} className="library-selection-bar">
      <Text size="caption" tone="secondary">{tr('{count} selected').replace('{count}', String(count))}</Text>
      <Spacer />
      <Button kind="secondary" onClick={onDownload} disabled={busy}>{tr('Download selected')}</Button>
      <Button kind="secondary" onClick={onDelete} disabled={busy}>{tr('Delete selected')}</Button>
      <IconButton title={tr('Clear selection')} icon="xmark" onClick={onClear} disabled={busy} />
    </HStack>
  )
}
