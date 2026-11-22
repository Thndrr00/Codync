import { useEffect, useState } from 'react'
import { isImageName, isVideoName, type Attachment, type SharedFile } from '@shared/models'
import { Button } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { AnchoredMenu, useContextMenu, type MenuItem } from '../../components/Menu'
import { IconButtonBase, ModalHeader, Sheet, SheetPage } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { copyPicture } from '../../lib/copy-image'
import { useTr } from '../../lib/use-language'
import { useStoreRef } from '../../store/context'
import type { BotStore } from '../../store/bot-store'
import './attachments.css'

/** The symbol for a file by its extension. */
export function attachmentSymbol(name: string) {
  const ext = name.split('.').pop()?.toLowerCase() ?? ''
  if (['png', 'jpg', 'jpeg', 'heic', 'gif', 'webp', 'tiff', 'bmp', 'svg'].includes(ext)) return 'photo'
  if (ext === 'pdf') return 'doc.richtext'
  if (['zip', 'gz', 'tar', '7z'].includes(ext)) return 'doc.zipper'
  if (['mov', 'mp4', 'm4v'].includes(ext)) return 'film'
  if (['mp3', 'm4a', 'wav', 'aac'].includes(ext)) return 'waveform'
  return 'doc'
}

export function byteCount(bytes: number) {
  if (bytes < 1000) return `${bytes} bytes`
  const units = ['KB', 'MB', 'GB']
  let value = bytes / 1000
  let unit = 0
  while (value >= 1000 && unit < units.length - 1) {
    value /= 1000
    unit++
  }
  return `${value < 10 ? value.toFixed(1) : Math.round(value)} ${units[unit]}`
}

/** Files sent with a message: images as pictures (click for full size), other files as cards. */
export function AttachmentList({ attachments, botId }: { attachments: Attachment[]; botId: string }) {
  const [viewing, setViewing] = useState<Attachment | null>(null)
  return (
    <VStack align="flex-end" gap={4}>
      {attachments.map((file) =>
        isImageName(file.name) ? (
          <ImageChip key={file.id} file={file} botId={botId} open={() => setViewing(file)} />
        ) : (
          <HStack key={file.id} gap={8} className="attachment-file">
            <Icon name={attachmentSymbol(file.name)} size={16} color="var(--secondary)" />
            <VStack gap={1} shrink>
              <Text size="subheadline" tone="text" truncate className="attachment-file-name">{file.name}</Text>
              <Text size="caption2" tone="tertiary">{byteCount(file.size)}</Text>
            </VStack>
          </HStack>
        ),
      )}
      <AttachmentViewer source={viewing && { file: viewing, botId }} close={() => setViewing(null)} />
    </VStack>
  )
}

/** Where a picture's bytes are on the computer: a file sent with a message, or one a bot shared. */
export type ImageSource = { file: Attachment; botId: string } | { file: SharedFile; entryId: string }

/** Copies an attachment's bytes through the same store path and clipboard helper as the viewer's Copy. */
export async function copyImage(store: BotStore, source: ImageSource): Promise<boolean> {
  const bytes = 'entryId' in source
    ? await store.sharedFileData(source.file, source.entryId)
    : await store.attachmentData(source.file.id, source.botId)
  if (!bytes) return false
  await copyPicture(new Uint8Array(bytes))
  return true
}

/** A picture on a message: click for full size, right-click to copy it (OpenBot 0.67.0). */
function ImageChip({ file, botId, open }: { file: Attachment; botId: string; open: () => void }) {
  const store = useStoreRef()
  const menu = useContextMenu()
  const source: ImageSource = { file, botId }
  const items = (): MenuItem[] => [{ title: 'Copy image', icon: 'square.on.square', action: () => void copyImage(store, source) }]
  return (
    <>
      <button className="attachment-image-button" aria-label={file.name} onClick={open} onContextMenu={menu.onContextMenu}>
        <AttachmentImage source={source} fit="thumbnail" />
      </button>
      <AnchoredMenu open={menu.open} onClose={menu.close} point={menu.point} items={items} />
    </>
  )
}

/** A picture or a clip full size, in a sheet, with zoom and fit; Escape steps back to fit before closing. */
export function AttachmentViewer({ source, close }: { source: ImageSource | null; close: () => void }) {
  const store = useStoreRef()
  const tr = useTr()
  const [note, setNote] = useState<string | null>(null)
  const [zoom, setZoom] = useState(1)
  useEffect(() => setZoom(1), [source?.file.id])
  const video = !!source && isVideoName(source.file.name) && 'entryId' in source
  const step = (delta: number) => setZoom((value) => Math.min(4, Math.max(0.5, Math.round((value + delta) * 100) / 100)))
  // Sheet owns Escape. Giving it a close that first resets zoom keeps the keyboard useful in the viewer.
  const onClose = () => { if (zoom !== 1) setZoom(1); else close() }
  const copy = async () => {
    if (!source) return
    try {
      if (!await copyImage(store, source)) return setNote(tr('The picture could not be copied'))
      setNote(tr('Copied'))
    } catch (reason) {
      setNote(reason instanceof Error ? reason.message : tr('The picture could not be copied'))
    }
  }
  return (
    <Sheet open={source !== null} onClose={onClose} width={1040} height={800}>
      {source ? (
        <SheetPage>
          <ModalHeader
            title={source.file.name}
            trailing={
              <HStack gap={6}>
                <Button kind="secondary" title="Zoom out" onClick={() => step(-0.25)}>−</Button>
                <Text size="caption" tone="secondary" tabular>{Math.round(zoom * 100)}%</Text>
                <Button kind="secondary" title="Zoom in" onClick={() => step(0.25)}>+</Button>
                <Button kind="secondary" title="Fit to window" onClick={() => setZoom(1)}>{tr('Fit')}</Button>
                <Button kind="secondary" onClick={() => void copy()}>{tr('Copy')}</Button>
              </HStack>
            }
          />
          {note ? <Text size="caption" tone="secondary">{note}</Text> : null}
          <div className="attachment-viewer">
            <div className="attachment-viewer-zoom" style={{ transform: `scale(${zoom})` }}>
              {video && 'entryId' in source
                ? <SharedVideo file={source.file} entryId={source.entryId} full />
                : <AttachmentImage source={source} fit="full" />}
            </div>
          </div>
        </SheetPage>
      ) : null}
    </Sheet>
  )
}

/** A picture from the computer, fetched once and then read from the cache. */
export function AttachmentImage({ source, fit }: { source: ImageSource; fit: 'icon' | 'thumbnail' | 'full' }) {
  const store = useStoreRef()
  const [url, setUrl] = useState<string | null>(null)
  useEffect(() => {
    let revoked = false
    let made: string | null = null
    const read = () => {
      const data = 'entryId' in source ? store.sharedFileData(source.file, source.entryId) : store.attachmentData(source.file.id, source.botId)
      void data.then((data) => {
        if (!data || revoked || made) return
        made = URL.createObjectURL(new Blob([data as BlobPart], { type: source.file.name.toLowerCase().endsWith('.svg') ? 'image/svg+xml' : '' }))
        setUrl(made)
      })
    }
    // A chat shown before the computer connects reads its pictures again once it comes online.
    let online = store.connection.kind === 'online'
    const stop = store.subscribe(() => {
      const now = store.connection.kind === 'online'
      if (now && !online && !made) read()
      online = now
    })
    read()
    return () => {
      revoked = true
      stop()
      if (made) URL.revokeObjectURL(made)
    }
    // A file's id names its bytes for good; `source` is a new object every render.
  }, [store, source.file.id])
  if (!url) {
    return (
      <div className={`attachment-placeholder ${fit}`}>
        <Icon name="photo" size={13} color="var(--tertiary)" />
      </div>
    )
  }
  return <img src={url} alt="" className={`attachment-image ${fit}`} />
}

/** What a clip's bytes are, by its extension, so the browser can play them. */
const videoType = (name: string) => {
  const types: Record<string, string> = { mov: 'video/quicktime', mp4: 'video/mp4', m4v: 'video/x-m4v', webm: 'video/webm' }
  return types[name.split('.').pop()?.toLowerCase() ?? ''] ?? 'video/mp4'
}

/** A clip a bot made, played in place or in the zoomable viewer; its bytes come from the computer like a picture's. */
export function SharedVideo({ file, entryId, full = false }: { file: SharedFile; entryId: string; full?: boolean }) {
  const store = useStoreRef()
  const tr = useTr()
  const state = store.fileDownloads.states.get(file.id)
  const busy = state?.kind === 'downloading'
  const download = () => busy ? store.fileDownloads.cancel(file.id) : void store.fileDownloads.save(store.client, entryId, file)
  const [url, setUrl] = useState<string | null>(null)
  useEffect(() => {
    let revoked = false
    let made: string | null = null
    const read = () => {
      void store.sharedFileData(file, entryId).then((data) => {
        if (!data || revoked || made) return
        made = URL.createObjectURL(new Blob([data as BlobPart], { type: videoType(file.name) }))
        setUrl(made)
      })
    }
    // A chat shown before the computer connects reads its clips again once it comes online.
    let online = store.connection.kind === 'online'
    const stop = store.subscribe(() => {
      const now = store.connection.kind === 'online'
      if (now && !online && !made) read()
      online = now
    })
    read()
    return () => {
      revoked = true
      stop()
      if (made) URL.revokeObjectURL(made)
    }
    // A file's id names its bytes for good; `file` is a new object once the entry is read again.
  }, [store, file.id, entryId])
  if (!url) {
    return (
      <div className="attachment-placeholder video">
        <Icon name="film" size={13} color="var(--tertiary)" />
      </div>
    )
  }
  // The browser's own controls keep time, size and the other buttons; Download sits on the
  // player itself (not buried in the right-click menu) alongside them (OpenBot 0.70.0).
  return (
    <div className={full ? 'video-player full' : 'video-player'}>
      <video src={url} className={full ? 'attachment-video full' : 'attachment-video'} controls playsInline preload="metadata" />
      <IconButtonBase className="video-download" size={30} title={busy ? tr('Cancel download') : tr('Download')}
        icon={busy ? 'xmark' : 'arrow.down.to.line'} onClick={download} />
    </div>
  )
}
