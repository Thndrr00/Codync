import { useCallback, useEffect, useState } from 'react'
import { bytesText } from '@shared/health'
import type { ModelDownload, ModelFile, ModelLibrary } from '@shared/tools'
import { Button, CardSection, ProgressBar, Spinner } from '../../../components/Controls'
import { Dialog } from '../../../components/Overlay'
import { HStack, VStack } from '../../../components/Stack'
import { Text } from '../../../components/Text'
import { useTr } from '../../../lib/use-language'
import { ErrorText } from '../pieces'
import { megabytes, type ToolCall } from './parts'

/** `upscale_models` → `Upscale models`. */
function folderName(folder: string): string {
  const words = folder.replaceAll('_', ' ')
  return words.charAt(0).toUpperCase() + words.slice(1)
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

function downloadOf(library: ModelLibrary | null, model: ModelFile, folder: string): ModelDownload | undefined {
  return library?.downloads.find((entry) => entry.folder === folder && entry.file === model.file)
}

/** One file: what it is, who needs it, and the action its state allows. */
function ModelRow({ model, download, busy, onDownload, onPause, onDelete }: {
  model: ModelFile
  download?: ModelDownload
  busy: boolean
  onDownload: () => void
  onPause: () => void
  onDelete: () => void
}) {
  const tr = useTr()
  const active = download?.phase === 'downloading'
  const percent = download && download.total > 0 ? download.done / download.total : 0
  return (
    <VStack gap={6}>
      <HStack gap={8}>
        <Text tone="text" grow>{model.file}</Text>
        <Text size="caption" tone="secondary">{bytesText(model.size)}</Text>
      </HStack>
      <HStack gap={8} wrap>
        {model.usedBy.length > 0 ? (
          <Text size="caption" tone="secondary">{tr('Needed by {names}').replace('{names}', model.usedBy.join(', '))}</Text>
        ) : model.present ? (
          <Text size="caption" tone="secondary">{tr('Added by hand')}</Text>
        ) : null}
        {model.license ? <Text size="caption" tone="secondary">{model.license}</Text> : null}
      </HStack>
      {active || download?.phase === 'paused' ? (
        <HStack gap={8}>
          <div className="comfy-progress">
            <ProgressBar value={percent} label={tr('Downloading {name}').replace('{name}', model.file)} />
          </div>
          <Text size="caption" tone="secondary">{`${Math.round(percent * 100)}%`}</Text>
          <Button kind="secondary" onClick={active ? onPause : onDownload}>{active ? tr('Pause') : tr('Resume')}</Button>
        </HStack>
      ) : null}
      {download?.phase === 'failed' ? <Text size="caption" tone="attention">{download.error}</Text> : null}
      <HStack gap={8}>
        {!model.present && !active ? (
          <Button kind="secondary" disabled={busy || !model.url} onClick={onDownload}>{tr(download?.phase === 'paused' ? 'Resume' : 'Download')}</Button>
        ) : null}
        {model.present ? <Button kind="secondary" disabled={busy} onClick={onDelete}>{tr('Delete')}</Button> : null}
        {!model.url && !model.present ? <Text size="caption" tone="secondary">{tr('No download URL in its manifest.')}</Text> : null}
      </HStack>
      {!active && download?.phase !== 'paused' && download?.phase !== 'failed' && model.size > 0 && !model.present ? (
        <Text size="caption" tone="secondary">{megabytes(model.size)}</Text>
      ) : null}
    </VStack>
  )
}

/** Models: what is installed, what a template still needs, and downloads that pause and resume. */
export function ComfyModels({ call }: { call: ToolCall }) {
  const tr = useTr()
  const [library, setLibrary] = useState<ModelLibrary | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [removing, setRemoving] = useState<{ folder: string; file: string; usedBy: string[] } | null>(null)

  const load = useCallback(async () => {
    try {
      setLibrary(await call<ModelLibrary>('comfyModels', {}, 30_000))
    } catch (e) {
      setError(message(e))
    }
  }, [call])

  useEffect(() => {
    void load()
  }, [load])

  useEffect(() => {
    if (!library?.downloads.some((entry) => entry.phase === 'downloading')) return
    const timer = setInterval(() => void load(), 1500)
    return () => clearInterval(timer)
  }, [library, load])

  const download = async (folder: string, file: string) => {
    setBusy(true)
    setError(null)
    try {
      await call('comfyModelDownload', { folder, file })
      await load()
    } catch (e) {
      setError(message(e))
    } finally {
      setBusy(false)
    }
  }

  const pause = async (folder: string, file: string) => {
    setError(null)
    try {
      await call('comfyModelPause', { folder, file })
      await load()
    } catch (e) {
      setError(message(e))
    }
  }

  const remove = async (folder: string, file: string, force: boolean) => {
    setBusy(true)
    setError(null)
    try {
      await call('comfyModelDelete', { folder, file, force })
      setRemoving(null)
      await load()
    } catch (e) {
      setError(message(e))
    } finally {
      setBusy(false)
    }
  }

  if (!library) {
    return <HStack gap={8}><Spinner size={18} /><Text tone="secondary">{tr('Reading the models…')}</Text></HStack>
  }

  const folders = library.folders.filter((folder) => folder.models.length > 0)
  return (
    <VStack gap={12}>
      <Text size="caption" tone="secondary">
        {tr('{used} of models in {free} free').replace('{used}', bytesText(library.totalBytes)).replace('{free}', bytesText(library.freeBytes))}
      </Text>
      {folders.length === 0 ? <Text size="caption" tone="secondary">{tr('No models yet. A template offers its own download.')}</Text> : null}
      {folders.map((folder) => (
        <CardSection key={folder.folder} title={tr(folderName(folder.folder))} footer={bytesText(folder.bytes)}>
          {folder.models.map((model) => (
            <ModelRow
              key={`${folder.folder}/${model.file}`}
              model={model}
              download={downloadOf(library, model, folder.folder)}
              busy={busy}
              onDownload={() => void download(folder.folder, model.file)}
              onPause={() => void pause(folder.folder, model.file)}
              onDelete={() => {
                if (model.usedBy.length > 0) setRemoving({ folder: folder.folder, file: model.file, usedBy: model.usedBy })
                else void remove(folder.folder, model.file, false)
              }}
            />
          ))}
        </CardSection>
      ))}
      {error ? <ErrorText text={error} /> : null}
      <Dialog
        open={removing !== null}
        title="Delete this model?"
        message={removing ? tr('{name} is needed by {names}. That template will not run until it is downloaded again.').replace('{name}', removing.file).replace('{names}', removing.usedBy.join(', ')) : null}
        actions={[{ title: 'Delete', destructive: true, action: () => removing && void remove(removing.folder, removing.file, true) }]}
        onCancel={() => setRemoving(null)}
        onClose={() => setRemoving(null)}
      />
    </VStack>
  )
}
