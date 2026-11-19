import { Icon } from '../../components/Icon'
import { VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import type { LibrarySection } from '../../lib/library-folders.ts'
import { useTr } from '../../lib/use-language'

/** The Library's side rail: this device's Favorites, then All files with Recent and Not in a folder under it. */
export function LibraryRail({ section, select, dragging, dropUnfiled }: {
  section: LibrarySection
  select: (section: LibrarySection) => void
  dragging: boolean
  dropUnfiled: () => void
}) {
  const tr = useTr()
  const item = (id: LibrarySection, label: string, icon: string) => (
    <button
      key={id}
      className={`library-rail-item${section === id ? ' active' : ''}${dragging && id === 'unfiled' ? ' drop-target' : ''}`}
      onClick={() => select(id)}
      onDragOver={(e) => {
        if (id !== 'unfiled' || !dragging) return
        e.preventDefault()
      }}
      onDrop={(e) => {
        if (id !== 'unfiled') return
        e.preventDefault()
        dropUnfiled()
      }}
      aria-current={section === id ? 'true' : undefined}
    >
      <Icon name={icon} size={14} color={section === id ? 'var(--text)' : 'var(--secondary)'} />
      <Text size="caption" weight={section === id ? 'semibold' : 'regular'}>{tr(label)}</Text>
    </button>
  )
  return (
    <VStack gap={2} className="library-rail">
      {item('favorites', 'Favorites', 'star')}
      {item('all', 'All files', 'tray')}
      <div className="library-rail-sub">{item('recent', 'Recent', 'clock')}{item('unfiled', 'Not in a folder', 'tray.and.arrow.up')}</div>
    </VStack>
  )
}
