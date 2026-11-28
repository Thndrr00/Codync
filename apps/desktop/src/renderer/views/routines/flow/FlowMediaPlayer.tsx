import { useEffect, useState } from 'react'
import { Spinner } from '../../../components/Controls'
import { useStore } from '../../../store/context'
import { audioMime, isAudioName } from '../../thread/audio'
import { isVideoName } from '@shared/models'

/**
 * Plays a media result from a workflow step. Unlike a chat file there is no chat entry to read
 * through; the host exposes `readFileById` scoped to the owning bot (OpenBot canvas media playback).
 */
export function FlowMediaPlayer({ botId, file }: { botId: string; file: { id: string; name: string; size: number } }) {
  const store = useStore()
  const [url, setUrl] = useState<string | null>(null)
  const [error, setError] = useState(false)

  useEffect(() => {
    let live = true
    let objectUrl: string | null = null
    const client = store.client
    if (!client) { setError(true); return }
    const read = async () => {
      try {
        const chunks: Uint8Array[] = []
        for (let offset = 0; offset < file.size; offset += 256 * 1024) {
          const reply = await client.call<{ data: string }>('readFileById', {
            botId, fileId: file.id, size: file.size, offset,
          })
          const binary = atob(reply.data)
          const bytes = new Uint8Array(binary.length)
          for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i)
          chunks.push(bytes)
        }
        if (!live) return
        const all = new Uint8Array(chunks.reduce((n, c) => n + c.length, 0))
        let at = 0
        for (const chunk of chunks) { all.set(chunk, at); at += chunk.length }
        objectUrl = URL.createObjectURL(new Blob([all as BlobPart], {
          type: isAudioName(file.name) ? audioMime(file.name) : 'video/mp4',
        }))
        setUrl(objectUrl)
      } catch {
        if (live) setError(true)
      }
    }
    void read()
    return () => { live = false; if (objectUrl) URL.revokeObjectURL(objectUrl) }
  }, [store.client, botId, file.id, file.name, file.size])

  if (error) return <div className="flow-media-error">Could not load this file.</div>
  if (!url) return <Spinner size={14} />
  return isVideoName(file.name)
    ? <video className="flow-media-player" src={url} controls preload="metadata" aria-label={file.name} />
    : <audio className="flow-media-player" src={url} controls preload="metadata" aria-label={file.name} />
}
