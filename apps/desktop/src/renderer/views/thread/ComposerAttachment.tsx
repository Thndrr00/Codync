import { useEffect, useState } from 'react'
import { isImageName } from '@shared/models'
import { Icon } from '../../components/Icon'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import type { OutgoingFile } from '../../store/bot-store'
import { useStore } from '../../store/context'
import { attachmentSymbol, byteCount } from './Attachments'

/**
 * A local attachment, shown before it is sent: images as a whole-image thumbnail with the
 * remove button in the corner, anything else (or an image that won't decode) as a name chip.
 * Its upload starts when it is attached: a ring shows the bytes the computer has and cancels it,
 * and a file that failed stays with its reason and a Retry.
 */
export function ComposerAttachment({ file, botId, remove }: { file: OutgoingFile; botId: string; remove: () => void }) {
  const store = useStore()
  const tr = useTr()
  const upload = store.uploads.state(file.id)
  const [url, setUrl] = useState<string | null>(null)
  const [broken, setBroken] = useState(false)

  useEffect(() => {
    if (!isImageName(file.name)) return
    const objectURL = URL.createObjectURL(new Blob([file.data as BlobPart]))
    setUrl(objectURL)
    return () => URL.revokeObjectURL(objectURL)
  }, [file])

  const uploading = upload?.status === 'uploading'
  const failed = upload?.status === 'failed' || upload?.status === 'cancelled'
  const percent = upload && upload.total > 0 ? Math.floor((upload.sent * 100) / upload.total) : 0
  const ring = uploading
    ? <UploadRing percent={percent} onImage={!!(url && !broken)} cancel={() => store.uploads.cancel(file.id)} />
    : null
  const reason = upload?.status === 'cancelled' ? tr('The upload was cancelled.') : (upload?.error ?? tr('Upload failed'))
  /** How much of it the computer has: its share and the bytes, as the ring shows them. */
  const line = `${percent}% · ${byteCount(upload?.sent ?? 0)} of ${byteCount(upload?.total ?? file.data.length)}`
  const removeButton = (
    <button
      className={url && !broken ? 'press attachment-remove' : 'file-chip-remove'}
      aria-label={tr('Remove {name}').replaceAll('{name}', () => file.name)}
      title={tr('Remove')}
      onClick={remove}
    >
      <Icon name="xmark" size={url && !broken ? 11 : 10} weight="bold" color={url && !broken ? '#fff' : 'var(--secondary)'} />
    </button>
  )

  return (
    <span className="attachment">
      {url && !broken ? (
        <span
          className="attachment-tile"
          aria-label={tr('Image attachment: {name}').replaceAll('{name}', () => file.name)}
          title={file.name}
        >
          <img src={url} alt="" onError={() => setBroken(true)} />
          {ring}
          {removeButton}
        </span>
      ) : (
        <span className="file-chip" title={file.name}>
          <Icon name={attachmentSymbol(file.name)} size={10} color={failed ? 'var(--danger)' : 'var(--secondary)'} />
          <Text size="footnote" tone="text" truncate className="file-chip-name">{file.name}</Text>
          {ring}
          {removeButton}
        </span>
      )}
      {uploading ? (
        <Text size="caption2" tone="secondary" tabular truncate className="attachment-line">{line}</Text>
      ) : null}
      {failed ? (
        <span className="attachment-error" aria-live="polite">
          <Text size="caption2" tone="danger" truncate className="attachment-error-text" title={reason}>{reason}</Text>
          <button className="press attachment-retry" aria-label={`${tr('Retry')} ${file.name}`} onClick={() => store.uploads.retry(botId, file)}>
            {tr('Retry')}
          </button>
        </span>
      ) : null}
    </span>
  )
}

/** The upload's progress as a ring around a stop mark: a click on it stops that file's upload. */
function UploadRing({ percent, onImage, cancel }: { percent: number; onImage: boolean; cancel: () => void }) {
  const tr = useTr()
  const radius = 8
  const circumference = 2 * Math.PI * radius
  const filled = (Math.min(100, Math.max(0, percent)) / 100) * circumference
  return (
    <button
      className="press attachment-ring"
      aria-label={`${tr('Cancel upload')} · ${percent}%`}
      title={`${tr('Cancel upload')} · ${percent}%`}
      onClick={cancel}
    >
      <svg className="attachment-ring-circle" viewBox="0 0 20 20" aria-hidden="true">
        <circle className="attachment-ring-track" cx="10" cy="10" r={radius} />
        <circle
          className="attachment-ring-fill"
          cx="10"
          cy="10"
          r={radius}
          strokeDasharray={`${filled} ${circumference}`}
        />
      </svg>
      <Icon name="xmark" size={7} weight="bold" color={onImage ? '#fff' : 'var(--text)'} />
    </button>
  )
}
