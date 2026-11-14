import { useEffect, useState } from 'react'
import { isImageName } from '@shared/models'
import { Icon } from '../../components/Icon'
import { Text } from '../../components/Text'
import type { OutgoingFile } from '../../store/outgoing-files'
import { useStore } from '../../store/context'
import { attachmentSymbol } from './Attachments'
import { useTr } from '../../lib/use-language'

/**
 * A local attachment, shown before it is sent: images as a whole-image thumbnail with the
 * remove button in the corner, anything else (or an image that won't decode) as a name chip.
 * Its upload starts when it is attached: a ring shows the bytes the computer has and cancels it,
 * and a file that failed stays with its reason and a Retry.
 */
export function ComposerAttachment({ file, botId, remove }: { file: OutgoingFile; botId: string; remove: () => void }) {
  const store = useStore()
  const upload = store.uploads.state(file.id)
  const tr = useTr()
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
  const ring = uploading ? <UploadRing percent={percent} cancel={() => store.uploads.cancel(file.id)} /> : null
  const removeButton = (
    <button
      className={url && !broken ? 'press attachment-remove' : 'file-chip-remove'}
      aria-label={`Remove ${file.name}`}
      title={tr('Remove')}
      onClick={remove}
    >
      <Icon name="xmark" size={url && !broken ? 11 : 10} weight="bold" color={url && !broken ? '#fff' : 'var(--secondary)'} />
    </button>
  )

  return (
    <span className={`attachment ${failed ? 'attachment-failed' : ''}`}>
      {url && !broken ? (
        <span className="attachment-tile" aria-label={tr('Image attachment: {name}').replaceAll('{name}', () => file.name)} title={file.name}>
          <img src={url} alt="" onError={() => setBroken(true)} />
          {ring}
          {removeButton}
        </span>
      ) : (
        <span className="file-chip">
          <Icon name={attachmentSymbol(file.name)} size={10} color="var(--secondary)" />
          <Text size="footnote" tone="text" truncate className="file-chip-name">{file.name}</Text>
          {uploading ? <Text size="caption2" tone="secondary" tabular>{`${percent}%`}</Text> : null}
          {ring}
          {removeButton}
        </span>
      )}
      {failed ? (
        <span className="attachment-error">
          <Text size="caption2" tone="danger" truncate>
            {tr(upload?.status === 'cancelled' ? 'Upload cancelled' : (upload?.error ?? 'Upload failed'))}
          </Text>
          <button className="press attachment-retry" onClick={() => store.uploads.retry(botId, file)}>
            {tr('Retry')}
          </button>
        </span>
      ) : null}
    </span>
  )
}

/** The upload's progress as a ring around a stop mark: a click on it stops that file's upload. */
function UploadRing({ percent, cancel }: { percent: number; cancel: () => void }) {
  const tr = useTr()
  const radius = 8
  const circumference = 2 * Math.PI * radius
  const filled = (Math.min(100, Math.max(0, percent)) / 100) * circumference
  return (
    <button className="press attachment-ring" aria-label={tr('Cancel upload')} title={tr('Cancel upload')} onClick={cancel}>
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
      <Icon name="xmark" size={7} weight="bold" color="#fff" />
    </button>
  )
}
