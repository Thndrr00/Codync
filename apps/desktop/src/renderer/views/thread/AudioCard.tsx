import { useEffect, useState } from 'react'
import { Icon } from '../../components/Icon'
import { IconButtonBase } from '../../components/Overlay'
import { HStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { useStore, useStoreRef } from '../../store/context'
import { audioMime } from './audio'
import type { ImageSource } from './Attachments'
import './attachments.css'

/**
 * A sound in the chat, read from the same store bytes as a picture and played by the browser's own
 * controls (OpenBot 0.70.0/0.72.0). A shared file also downloads from the player, like a shared clip.
 */
export function AudioCard({ source }: { source: ImageSource }) {
  const store = useStoreRef()
  const live = useStore()
  const tr = useTr()
  const [url, setUrl] = useState<string | null>(null)
  const shared = 'entryId' in source ? source : null
  const state = shared ? live.fileDownloads.states.get(shared.file.id) : undefined
  const busy = state?.kind === 'downloading'
  useEffect(() => {
    let revoked = false
    let made: string | null = null
    const read = () => {
      const data = shared
        ? store.sharedFileData(shared.file, shared.entryId)
        : store.attachmentData(source.file.id, 'botId' in source ? source.botId : '')
      void data.then((data) => {
        if (!data || revoked || made) return
        made = URL.createObjectURL(new Blob([data as BlobPart], { type: audioMime(source.file.name) }))
        setUrl(made)
      })
    }
    // A chat shown before the computer connects reads its sounds again once it comes online.
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
  }, [store, source.file.id, source.file.name, shared?.entryId])
  if (!url) {
    return (
      <HStack gap={6} className="attachment-file audio-loading">
        <Icon name="waveform" size={16} color="var(--secondary)" />
        <Text size="caption2" tone="secondary">{tr('Loading audio…')}</Text>
      </HStack>
    )
  }
  return (
    <HStack gap={4} className="audio-player">
      <audio src={url} controls preload="metadata" aria-label={source.file.name} />
      {shared ? (
        <IconButtonBase
          size={30}
          title={busy ? tr('Cancel download') : tr('Download')}
          icon={busy ? 'xmark' : 'arrow.down.to.line'}
          onClick={() => busy ? live.fileDownloads.cancel(shared.file.id) : void live.fileDownloads.save(live.client, shared.entryId, shared.file)}
        />
      ) : null}
    </HStack>
  )
}
