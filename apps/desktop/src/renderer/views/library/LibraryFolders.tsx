import type { LibraryItem } from '@shared/artifacts'
import { DropdownMenu, IconButton } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { libraryItemName, type LibraryFolder } from '../../lib/library-folders.ts'
import { useTr } from '../../lib/use-language'
import { TitleField } from './ArtifactParts'

/** The folder tiles above a view's files: newest items first, with a temporary New folder tile while dragging. */
export function LibraryFolders({ folders, items, favorites, dragging, renamingId, dropItem, dropNew, open, star, rename, move, remove, doneRenaming }: {
  folders: LibraryFolder[]
  items: LibraryItem[]
  favorites: readonly string[]
  dragging: boolean
  renamingId: string | null
  dropItem: (folderId: string, key: string) => void
  dropNew: (key: string) => void
  open: (folderId: string) => void
  star: (folderId: string) => void
  rename: (folderId: string) => void
  move: (folderId: string, beforeId: string | null) => void
  remove: (folderId: string) => void
  doneRenaming: (name: string | null) => void
}) {
  const tr = useTr()
  const names = new Map(items.map((item) => [item.kind === 'artifact' ? `artifact:${item.artifact.id}` : `${item.kind}:${item.entryId}:${item.file.id}`, libraryItemName(item)]))
  return (
    <div className="library-folder-grid">
      {folders.map((folder) => (
        <div
          key={folder.id}
          className={`library-folder-tile${dragging ? ' can-drop' : ''}`}
          onDragOver={(e) => { if (dragging) e.preventDefault() }}
          onDrop={(e) => { e.preventDefault(); const key = e.dataTransfer.getData('text/plain'); if (key) dropItem(folder.id, key) }}
        >
          <HStack gap={4} align="center">
            <button className="library-folder-open" onClick={() => open(folder.id)}>
              <Icon name="folder" size={16} color="var(--secondary)" />
              {renamingId === folder.id ? (
                <TitleField initial={folder.name} className="library-folder-field" done={doneRenaming} />
              ) : (
                <>
                  <Text size="callout" weight="semibold" truncate title={folder.name}>{folder.name}</Text>
                  <Text size="caption2" tone="tertiary">{folder.count}</Text>
                </>
              )}
            </button>
            <IconButton
              title={favorites.includes(folder.id) ? tr('Unstar folder') : tr('Star folder')}
              icon={favorites.includes(folder.id) ? 'star.fill' : 'star'}
              onClick={() => star(folder.id)}
            />
            <DropdownMenu
              className="icon-button library-folder-menu"
              title={tr('More actions')}
              items={() => [
                { title: 'Rename folder', icon: 'pencil', action: () => rename(folder.id) },
                { title: 'Move to front', icon: 'arrow.up', action: () => move(folder.id, folders[0]?.id === folder.id ? null : folders[0]?.id ?? null) },
                { title: 'Move to back', icon: 'arrow.down', action: () => move(folder.id, null) },
                { title: 'Delete folder', icon: 'trash', action: () => remove(folder.id) },
              ]}
            >
              <Icon name="ellipsis" size={14} weight="medium" />
            </DropdownMenu>
          </HStack>
          <VStack gap={1} className="library-folder-newest">
            {folder.items.slice(0, 4).map((key) => (
              <Text key={key} size="caption2" tone="tertiary" truncate title={names.get(key) ?? key}>{names.get(key) ?? key}</Text>
            ))}
          </VStack>
        </div>
      ))}
      {dragging ? (
        <div
          className="library-folder-tile new"
          onDragOver={(e) => e.preventDefault()}
          onDrop={(e) => { e.preventDefault(); const key = e.dataTransfer.getData('text/plain'); if (key) dropNew(key) }}
        >
          <HStack gap={6} align="center">
            <Icon name="plus" size={16} color="var(--secondary)" />
            <Text size="callout" tone="secondary">{tr('New folder')}</Text>
          </HStack>
        </div>
      ) : null}
    </div>
  )
}
