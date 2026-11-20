import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react'
import type { Artifact, LibraryItem } from '@shared/artifacts'
import { isImageName, type Attachment, type SharedFile } from '@shared/models'
import { DropdownMenu, IconButton } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { inSection, libraryItemKey, readFavorites, toggleFavorite, visibleFolders, type LibraryFolder, type LibrarySection } from '../../lib/library-folders.ts'
import { libraryGroups } from '../../lib/library-groups'
import { trNow } from '../../lib/l10n.ts'
import { useTr } from '../../lib/use-language'
import { useScreenViewed } from '../../store/analytics'
import { useStore } from '../../store/context'
import { AttachmentImage, AttachmentViewer, attachmentSymbol, byteCount } from '../thread/Attachments'
import { downloadAction, downloadDetail } from '../thread/FileDownloadCard'
import { artifactShareItems, renameArtifact, TitleField } from './ArtifactParts'
import { LibraryFolders } from './LibraryFolders'
import { LibraryRail } from './LibraryRail'
import { LibraryState } from './LibraryState'
import { LibraryUndo } from './LibraryUndo'
import './library.css'

/**
 * A chat's Library in its details panel (Grok Bot's): the artifacts its bot made and the files shared in it, newest
 * first, by day. Each opens the way it does in the chat. A side rail filters it and its folders; items can be filed
 * by dragging them onto a folder tile, or onto New folder / Not in a folder.
 */
export function LibraryView({ botId, openArtifact }: { botId: string; openArtifact: (artifactId: string) => void }) {
  const tr = useTr()
  const store = useStore()
  const [items, setItems] = useState<LibraryItem[]>([])
  const [folders, setFolders] = useState<LibraryFolder[]>([])
  const [section, setSection] = useState<LibrarySection>('all')
  const [favorites, setFavorites] = useState<string[]>(() => readFavorites(botId))
  const [notice, setNotice] = useState<{ text: string; undo: () => void } | null>(null)
  const [renamingId, setRenamingId] = useState<string | null>(null)
  const [dragging, setDragging] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)
  const [viewing, setViewing] = useState<Attachment | null>(null)
  const generation = useRef(0)
  const pendingKey = useRef('')
  useScreenViewed('library')
  // Read again when the chat gets an artifact card or a file.
  const shared = store.allEntries(botId).filter((e) => e.data.artifact || e.data.files?.length || e.data.attachments?.length).length

  const refresh = useCallback(() => {
    const mine = ++generation.current
    setLoading(true)
    store.artifacts.library(store.client, botId).then(
      (items) => { if (mine === generation.current) (setItems(items), setLoadError(null), setLoading(false)) },
      (e: unknown) => { if (mine === generation.current) (setLoadError(e instanceof Error ? e.message : String(e)), setLoading(false)) },
    )
    // An older host has no folder method: keep the library as it was, with no folders.
    store.client?.call<{ folders: LibraryFolder[] }>('libraryFolders', { botId }).then(
      (result) => { if (mine === generation.current) setFolders(result.folders) },
      () => { if (mine === generation.current) setFolders([]) },
    )
  }, [store, botId])

  useEffect(() => { refresh() }, [refresh, shared])
  // A bot can reorganize its folders while this view is open; pick up those changes too.
  useEffect(() => {
    const timer = setInterval(refresh, 15000)
    return () => clearInterval(timer)
  }, [refresh])
  useEffect(() => {
    setSection('all'); setNotice(null); setRenamingId(null); setFavorites(readFavorites(botId))
  }, [botId])

  const showError = (e: unknown) => setError(e instanceof Error ? e.message : String(e))
  const call = useCallback(async (method: string, body: object) => {
    if (!store.client) throw new Error(trNow('Connect to this computer to change its library'))
    const result = await store.client.call<{ folders: LibraryFolder[] }>(method, { botId, ...body })
    setFolders(result.folders)
    return result
  }, [store.client, botId])

  /** Show the undo notice for a finished change; `undo` runs the inverse operation from this device. */
  const changed = (text: string, undo: () => void) => {
    setNotice({ text, undo })
    refresh()
  }
  const settle = () => { setNotice(null); refresh() }

  const fileItem = (key: string, folderId: string | null) => {
    const previous = items.find((item) => libraryItemKey(item) === key)?.folderId ?? null
    const label = folderId ? folders.find((folder) => folder.id === folderId)?.name ?? '' : tr('Not in a folder')
    void call('libraryFileItem', folderId ? { item: key, folderId } : { item: key }).then(
      () => changed(tr(folderId ? 'Moved to {folder}' : 'Taken out of folders').replace('{folder}', label), () => {
        void call('libraryFileItem', previous ? { item: key, folderId: previous } : { item: key }).then(settle, showError)
      }),
      showError,
    )
  }

  const dropNew = (key: string) => {
    void call('libraryFolderCreate', { name: tr('New folder') }).then((result) => {
      const created = result.folders[result.folders.length - 1]
      if (!created) return
      void call('libraryFileItem', { item: key, folderId: created.id }).then(
        () => {
          setRenamingId(created.id)
          changed(tr('Moved to {folder}').replace('{folder}', created.name), () => {
            void call('libraryFolderDelete', { id: created.id }).then(settle, showError)
          })
        },
        showError,
      )
    }, showError)
  }

  const rename = (id: string, name: string | null) => {
    setRenamingId(null)
    if (!name) return
    const previous = folders.find((folder) => folder.id === id)?.name ?? ''
    void call('libraryFolderRename', { id, name }).then(
      () => changed(tr('Renamed folder to {folder}').replace('{folder}', name), () => {
        void call('libraryFolderRename', { id, name: previous }).then(settle, showError)
      }),
      showError,
    )
  }

  const move = (id: string, beforeId: string | null) => {
    const folder = folders.find((f) => f.id === id)
    const at = folders.findIndex((f) => f.id === id)
    const previousBefore = folders[at + 1]?.id ?? null
    void call('libraryFolderMove', { id, beforeId }).then(
      () => changed(tr('Moved folder {folder}').replace('{folder}', folder?.name ?? ''), () => {
        void call('libraryFolderMove', { id, beforeId: previousBefore }).then(settle, showError)
      }),
      showError,
    )
  }

  const remove = (id: string) => {
    const folder = folders.find((f) => f.id === id)
    if (!folder) return
    void call('libraryFolderDelete', { id: folder.id }).then(
      () => changed(tr('Deleted folder {folder}').replace('{folder}', folder.name), () => {
        void call('libraryFolderCreate', { name: folder.name }).then((result) => {
          const recreated = result.folders.find((f) => f.name === folder.name)
          if (!recreated) return settle()
          Promise.all(folder.items.map((key) => call('libraryFileItem', { item: key, folderId: recreated.id }))).then(settle, showError)
        }, showError)
      }),
      showError,
    )
  }

  const star = (id: string) => setFavorites(toggleFavorite(botId, id))
  const shown = items.filter((item) => inSection(item, section, favorites, Date.now()))
  const tiles = visibleFolders(folders, section, favorites)
  return (
    <HStack gap={12} align="flex-start" className="library-view">
      <LibraryRail section={section} select={setSection} dragging={dragging} dropUnfiled={() => { if (pendingKey.current) fileItem(pendingKey.current, null) }} />
      <VStack gap={12} grow className="library-content">
        {error ? <Text size="caption" tone="danger" role="alert">{error}</Text> : null}
        {loadError ? <LibraryState kind="error" title="Couldn’t load this library" detail={loadError} action="Retry" onAction={refresh} /> : null}
        {notice ? <LibraryUndo text={notice.text} undo={() => { setNotice(null); notice.undo() }} dismiss={() => setNotice(null)} /> : null}
        <LibraryFolders
          folders={tiles}
          items={items}
          favorites={favorites}
          dragging={dragging}
          renamingId={renamingId}
          dropItem={(folderId, key) => fileItem(key, folderId)}
          dropNew={dropNew}
          open={setSection}
          star={star}
          rename={setRenamingId}
          move={move}
          remove={remove}
          doneRenaming={(name) => renamingId && rename(renamingId, name)}
        />
        {!loadError && loading && items.length === 0 && folders.length === 0 ? (
          <LibraryState kind="loading" title="Loading your library…" />
        ) : !loadError && items.length === 0 && folders.length === 0 ? (
          <LibraryState
            kind="empty"
            title="Library is empty"
            detail={tr('Artifacts {name} makes and files shared in this chat show here.').replace('{name}', store.bots.get(botId)?.name ?? '')}
            action="Refresh"
            onAction={refresh}
          />
        ) : (
          libraryGroups(shown).map(({ group, items }) => (
            <section key={group} className="details-section">
              <Text className="details-section-title">{tr(group)}</Text>
              <VStack gap={6}>
                {items.map((item) =>
                  item.kind === 'artifact' ? (
                    <ArtifactRow key={item.artifact.id} artifact={store.artifacts.byId.get(item.artifact.id) ?? item.artifact} open={() => openArtifact(item.artifact.id)} onError={setError} itemKey={libraryItemKey(item)} dragging={setDragging} pending={pendingKey} />
                  ) : item.kind === 'file' ? (
                    <FileRow key={item.file.id} entryId={item.entryId} file={item.file} itemKey={libraryItemKey(item)} dragging={setDragging} pending={pendingKey} />
                  ) : (
                    <AttachmentRow key={item.file.id} botId={botId} file={item.file} view={() => setViewing(item.file)} itemKey={libraryItemKey(item)} dragging={setDragging} pending={pendingKey} />
                  ),
                )}
              </VStack>
            </section>
          ))
        )}
        <AttachmentViewer source={viewing && { file: viewing, botId }} close={() => setViewing(null)} />
      </VStack>
    </HStack>
  )
}

/** One line of the Library: what it is, its name, and what can be done with it. */
function Row({ leading, children, trailing, itemKey, dragging, pending }: {
  leading: ReactNode; children: ReactNode; trailing?: ReactNode
  itemKey?: string; dragging?: (value: boolean) => void; pending?: { current: string }
}) {
  return (
    <HStack
      gap={4}
      className="outlined-card library-row"
      draggable={!!itemKey}
      onDragStart={itemKey ? (e) => {
        e.dataTransfer.setData('text/plain', itemKey)
        e.dataTransfer.effectAllowed = 'move'
        if (pending) pending.current = itemKey
        dragging?.(true)
      } : undefined}
      onDragEnd={() => dragging?.(false)}
    >
      <HStack gap={10} grow className="library-row-main">
        <span className="library-row-icon">{leading}</span>
        {children}
      </HStack>
      {trailing}
    </HStack>
  )
}

function ArtifactRow({ artifact, open, onError, itemKey, dragging, pending }: {
  artifact: Artifact; open: () => void; onError: (message: string | null) => void
  itemKey?: string; dragging?: (value: boolean) => void; pending?: { current: string }
}) {
  const tr = useTr()
  const store = useStore()
  const [renaming, setRenaming] = useState(false)
  const menu = (
    <DropdownMenu
      className="icon-button library-row-menu"
      title={tr('More actions')}
      items={() => [
        { title: 'Open', icon: 'doc.text', action: open },
        { title: 'Rename artifact', icon: 'pencil', action: () => setRenaming(true) },
        ...artifactShareItems(store, artifact, onError),
      ]}
    >
      <Icon name="ellipsis" size={14} weight="medium" />
    </DropdownMenu>
  )
  const icon = <Icon name="chart.bar.doc.horizontal" size={16} color="var(--secondary)" />
  if (renaming) {
    return (
      <Row leading={icon} trailing={menu}>
        <TitleField
          initial={artifact.title}
          className="library-row-field"
          done={(title) => {
            setRenaming(false)
            renameArtifact(store, artifact, title, onError)
          }}
        />
      </Row>
    )
  }
  return (
    <Row leading={icon} trailing={menu} itemKey={itemKey} dragging={dragging} pending={pending}>
      <button className="library-row-open" onClick={open}>
        <Name name={artifact.title} detail={tr('Artifact')} />
      </button>
    </Row>
  )
}

/** A file the bot shared: saved like its card in the chat. */
function FileRow({ entryId, file, itemKey, dragging, pending }: {
  entryId: string; file: SharedFile
  itemKey?: string; dragging?: (value: boolean) => void; pending?: { current: string }
}) {
  const store = useStore()
  const state = store.fileDownloads.states.get(file.id)
  const busy = state?.kind === 'downloading'
  return (
    <Row
      leading={<Icon name={attachmentSymbol(file.name)} size={16} color="var(--secondary)" />}
      trailing={<IconButton title={downloadAction(state)} icon={busy ? 'xmark' : 'arrow.down.to.line'} onClick={() => (busy ? store.fileDownloads.cancel(file.id) : void store.fileDownloads.save(store.client, entryId, file))} />}
      itemKey={itemKey}
      dragging={dragging}
      pending={pending}
    >
      <Name name={file.name} detail={downloadDetail(state, file)} failed={state?.kind === 'failed'} />
    </Row>
  )
}

/** A file the user sent: an image opens full size, like in the chat. */
function AttachmentRow({ botId, file, view, itemKey, dragging, pending }: {
  botId: string; file: Attachment; view: () => void
  itemKey?: string; dragging?: (value: boolean) => void; pending?: { current: string }
}) {
  if (!isImageName(file.name)) {
    return (
      <Row leading={<Icon name={attachmentSymbol(file.name)} size={16} color="var(--secondary)" />} itemKey={itemKey} dragging={dragging} pending={pending}>
        <Name name={file.name} detail={byteCount(file.size)} />
      </Row>
    )
  }
  return (
    <Row leading={<AttachmentImage source={{ file, botId }} fit="icon" />} itemKey={itemKey} dragging={dragging} pending={pending}>
      <button className="library-row-open" onClick={view}>
        <Name name={file.name} detail={byteCount(file.size)} />
      </button>
    </Row>
  )
}

function Name({ name, detail, failed = false }: { name: string; detail: string; failed?: boolean }) {
  return (
    <VStack gap={1} shrink>
      <Text size="callout" truncate title={name}>{name}</Text>
      <Text size="caption2" tone={failed ? 'danger' : 'tertiary'} truncate>{detail}</Text>
    </VStack>
  )
}
