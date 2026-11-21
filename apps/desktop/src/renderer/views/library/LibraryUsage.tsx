import type { LibraryItem } from '@shared/artifacts'
import { ProgressBar } from '../../components/Controls'
import { HStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { byteCount } from '../thread/Attachments'
import './library.css'

/**
 * A soft visual ceiling for the bar only; the label always shows the bytes actually kept. There is
 * no quota call and no polling — the Library's own refresh already supplies the items.
 */
const DISPLAY_CEILING = 100 * 1024 * 1024

/** How much this chat has kept: its artifacts and every file shared or attached in it. */
export function LibraryUsage({ items }: { items: LibraryItem[] }) {
  const tr = useTr()
  const bytes = items.reduce((total, item) => total + (item.kind === 'artifact' ? item.artifact.size : item.file.size), 0)
  if (bytes <= 0) return null
  return (
    <div className="library-usage">
      <HStack gap={8}>
        <Text size="caption" tone="secondary">{tr('Storage used')}</Text>
        <Text size="caption" tone="tertiary">{byteCount(bytes)}</Text>
      </HStack>
      <ProgressBar value={Math.min(1, bytes / DISPLAY_CEILING)} label={tr('Storage used by this chat')} />
    </div>
  )
}
