import { useEffect, useState, type ReactNode } from 'react'
import type { ServerLibraryFilter, ServerLibraryItem } from '@shared/artifacts'
import { Button, ChoicePicker, IconButton, Spinner } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { ModalHeader, Sheet, SheetPage } from '../../components/Overlay'
import { HStack, Spacer, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { libraryGroups } from '../../lib/library-groups'
import { serverLibraryItems } from '../../lib/server-library'
import { useTr } from '../../lib/use-language'
import { useScreenViewed } from '../../store/analytics'
import type { BotStore } from '../../store/bot-store'
import { StoreContext, useApp, useStore } from '../../store/context'
import { ArtifactView } from './ArtifactView'
import { AttachmentImage, AttachmentViewer, attachmentSymbol, byteCount, type ImageSource } from '../thread/Attachments'
import { downloadAction, downloadDetail } from '../thread/FileDownloadCard'
import './library.css'

/**
 * A computer's server Library (Grok Bot's, server-wide): every bot's artifacts and files, newest first, by day. Each row
 * opens the way its chat card does; the filter only hides rows, the host already sent them newest first.
 */
export function ServerLibraryView() {
  const tr = useTr()
  const app = useApp()
  const [filter, setFilter] = useState<ServerLibraryFilter>('all')
  const [lists, setLists] = useState<Record<string, ServerLibraryItem[]>>({})
  const [errors, setErrors] = useState<Record<string, string>>({})
  const [loading, setLoading] = useState(true)
  const [refresh, setRefresh] = useState(0)
  const [viewing, setViewing] = useState<{ store: BotStore; source: ImageSource } | null>(null)
  const [artifact, setArtifact] = useState<{ store: BotStore; botId: string; id: string } | null>(null)
  useScreenViewed('library')
  const computers = app.computers.filter((computer) => app.store(computer.id)?.hasServerLibrary)
  const computerKey = computers.map((computer) => computer.id).join(',')

  // One read a computer, when the page opens, the computers change, or Refresh is pressed.
  useEffect(() => {
    let live = true
    setLoading(true)
    const reads = computers.map(async (computer) => {
      const store = app.store(computer.id)
      if (!store) return
      try {
        const items = await store.artifacts.serverLibrary(store.client)
        if (!live) return
        setLists((all) => ({ ...all, [computer.id]: items }))
        setErrors((all) => {
          const next = { ...all }
          delete next[computer.id]
          return next
        })
      } catch (e: unknown) {
        if (live) setErrors((all) => ({ ...all, [computer.id]: e instanceof Error ? e.message : String(e) }))
      }
    })
    void Promise.all(reads).finally(() => live && setLoading(false))
    return () => {
      live = false
    }
    // `computers` is read through `computerKey`: the same computers, with the page's own refresh.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [computerKey, refresh])

  const openArtifact = (store: BotStore, item: ServerLibraryItem) => {
    if (item.kind === 'artifact') setArtifact({ store, botId: item.botId, id: item.artifact.id })
  }

  return (
    <SheetPage className="server-library">
      <ModalHeader
        icon="books.vertical"
        title={tr('Server Library')}
        subtitle={tr("Every artifact and file from this computer's bots, newest first.")}
        trailing={
          <Button kind="secondary" disabled={loading} onClick={() => setRefresh((count) => count + 1)}>
            {loading ? tr('Refreshing…') : tr('Refresh')}
          </Button>
        }
      />
      <div className="server-library-body">
        <HStack gap={8} className="server-library-tools">
        <ChoicePicker
          selection={filter}
          options={[
            { id: 'all', label: 'All' },
            { id: 'images', label: 'Images' },
            { id: 'files', label: 'Files' },
          ]}
          onChange={setFilter}
        />
        <Spacer />
        {loading ? <Spinner size={14} /> : null}
      </HStack>
      {computers.length === 0 ? (
        <Empty title={tr('Library is empty')} detail={tr('Artifacts and files from every bot on this computer show here.')} />
      ) : (
        computers.map((computer) => {
          const store = app.store(computer.id)
          if (!store) return null
          const items = serverLibraryItems(lists[computer.id] ?? [], filter)
          return (
            <StoreContext.Provider key={computer.id} value={store}>
              <section className="details-section">
                <Text className="details-section-title">{store.hostName}</Text>
                {errors[computer.id] ? (
                  <Text size="caption" tone="danger" role="alert">{errors[computer.id]}</Text>
                ) : lists[computer.id] === undefined ? (
                  <Text size="callout" tone="secondary">{tr('Loading…')}</Text>
                ) : items.length === 0 ? (
                  <Empty
                    title={tr('Library is empty')}
                    detail={tr('Artifacts and files from every bot on this computer show here.')}
                  />
                ) : (
                  libraryGroups(items).map(({ group, items: groupItems }) => (
                    <VStack key={group} gap={6} className="server-library-group">
                      <Text size="caption" weight="semibold" tone="secondary">{tr(group)}</Text>
                      {groupItems.map((item) => (
                        <ServerRow
                          key={`${item.kind}/${item.botId}/${item.kind === 'artifact' ? item.artifact.id : item.file.id}`}
                          item={item}
                          openArtifact={() => item.kind === 'artifact' && openArtifact(store, item)}
                          view={(source) => setViewing({ store, source })}
                        />
                      ))}
                    </VStack>
                  ))
                )}
              </section>
            </StoreContext.Provider>
          )
        })
      )}
      </div>
      {viewing ? (
        <StoreContext.Provider value={viewing.store}>
          <AttachmentViewer source={viewing.source} close={() => setViewing(null)} />
        </StoreContext.Provider>
      ) : null}
      <Sheet open={artifact !== null} onClose={() => setArtifact(null)} width={1040} height={800}>
        {artifact ? (
          <StoreContext.Provider value={artifact.store}>
            <div className="artifact-tabbed">
              <ArtifactView botId={artifact.botId} artifactId={artifact.id} close={() => setArtifact(null)} />
            </div>
          </StoreContext.Provider>
        ) : null}
      </Sheet>
    </SheetPage>
  )
}

/** One row: the bot it came from, its name and what it is; an image opens full size, a file saves. */
function ServerRow({ item, openArtifact, view }: {
  item: ServerLibraryItem
  openArtifact: () => void
  view: (source: ImageSource) => void
}) {
  const tr = useTr()
  const store = useStore()
  const from = tr('From {name}').replace('{name}', item.botName)
  if (item.kind === 'artifact') {
    return (
      <Row leading={<Icon name="chart.bar.doc.horizontal" size={16} color="var(--secondary)" />}>
        <button className="library-row-open" onClick={openArtifact}>
          <Name name={item.artifact.title} detail={`${from} · ${tr('Artifact')}`} />
        </button>
      </Row>
    )
  }
  if (item.kind === 'file') {
    const source: ImageSource = { file: item.file, entryId: item.entryId }
    if (item.image) {
      return (
        <Row leading={<AttachmentImage source={source} fit="icon" />}>
          <button className="library-row-open" onClick={() => view(source)}>
            <Name name={item.file.name} detail={`${from} · ${tr('Image')}`} />
          </button>
        </Row>
      )
    }
    const state = store.fileDownloads.states.get(item.file.id)
    const busy = state?.kind === 'downloading'
    return (
      <Row
        leading={<Icon name={attachmentSymbol(item.file.name)} size={16} color="var(--secondary)" />}
        trailing={<IconButton title={downloadAction(state)} icon={busy ? 'xmark' : 'arrow.down.to.line'} onClick={() => (busy ? store.fileDownloads.cancel(item.file.id) : void store.fileDownloads.save(store.client, item.entryId, item.file))} />}
      >
        <Name name={item.file.name} detail={`${from} · ${downloadDetail(state, item.file)}`} failed={state?.kind === 'failed'} />
      </Row>
    )
  }
  const source: ImageSource = { file: item.file, botId: item.botId }
  if (item.image) {
    return (
      <Row leading={<AttachmentImage source={source} fit="icon" />}>
        <button className="library-row-open" onClick={() => view(source)}>
          <Name name={item.file.name} detail={`${from} · ${tr('Image')}`} />
        </button>
      </Row>
    )
  }
  return (
    <Row leading={<Icon name={attachmentSymbol(item.file.name)} size={16} color="var(--secondary)" />}>
      <Name name={item.file.name} detail={`${from} · ${byteCount(item.file.size)}`} />
    </Row>
  )
}

function Row({ leading, children, trailing }: { leading: ReactNode; children: ReactNode; trailing?: ReactNode }) {
  return (
    <HStack gap={4} className="outlined-card library-row">
      <HStack gap={10} grow className="library-row-main">
        <span className="library-row-icon">{leading}</span>
        {children}
      </HStack>
      {trailing}
    </HStack>
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

function Empty({ title, detail }: { title: string; detail: string }) {
  return (
    <VStack gap={4} className="outlined-card library-empty">
      <Text size="callout" weight="semibold">{title}</Text>
      <Text size="callout" tone="secondary">{detail}</Text>
    </VStack>
  )
}
