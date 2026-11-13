import { useCallback, useEffect, useState } from 'react'
import type { ToolJob } from '@shared/tools'
import { Button, CardSection, ProgressBar, Spinner } from '../../../components/Controls'
import { HStack, VStack } from '../../../components/Stack'
import { Text } from '../../../components/Text'
import { Icon } from '../../../components/Icon'
import { useTr } from '../../../lib/use-language'
import { ErrorText } from '../pieces'
import type { ToolCall } from './parts'

const PHASES: Record<ToolJob['phase'], string> = {
  queued: 'Waiting in the queue',
  running: 'Running',
  done: 'Done',
  cancelled: 'Cancelled',
  error: 'Failed',
}

function kindIcon(kind: string | undefined): string {
  return kind === 'video' ? 'film' : kind === 'audio' ? 'waveform' : kind === 'mesh' ? 'shippingbox' : 'photo'
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

function when(createdAt: number | undefined): string {
  return createdAt ? new Date(createdAt).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) : ''
}

/** One job: its phase, the step it is on, and the files it made with the type they really have. */
function JobCard({ job, onCancel }: { job: ToolJob; onCancel: () => void }) {
  const tr = useTr()
  const active = job.phase === 'queued' || job.phase === 'running'
  const progress = job.progress
  return (
    <VStack gap={8}>
      <HStack gap={8}>
        {active ? <Spinner size={14} /> : null}
        <Text tone={job.phase === 'error' ? 'attention' : 'text'} grow>
          {job.template ? tr(job.template) : tr('Workflow')}
        </Text>
        <Text size="caption" tone="secondary">{tr(PHASES[job.phase])}</Text>
      </HStack>
      {progress ? (
        <VStack gap={4}>
          <HStack gap={8}>
            <Text size="caption" tone="secondary" grow>
              {progress.class || tr('Node {node}').replace('{node}', progress.node)}
            </Text>
            {progress.total > 0 ? <Text size="caption" tone="secondary">{`${progress.step}/${progress.total}`}</Text> : null}
          </HStack>
          {progress.percent !== null ? <ProgressBar value={progress.percent} label={tr('Progress')} /> : null}
        </VStack>
      ) : null}
      {job.error ? <Text size="caption" tone="attention">{job.error}</Text> : null}
      {(job.outputs ?? []).map((output) => (
        <HStack key={output.path} gap={8}>
          <Icon name={kindIcon(output.kind)} size={14} color="var(--secondary)" />
          <Text size="caption" tone="secondary" grow>{output.path.split(/[\\/]/).pop()}</Text>
          <Text size="caption" tone="secondary">{output.mime}</Text>
        </HStack>
      ))}
      {active ? <Button kind="secondary" onClick={onCancel}>{tr('Cancel')}</Button> : null}
    </VStack>
  )
}

/** The current job with its real progress, the poster of what it made, and the recent jobs. */
export function ComfyJobs({ call, current }: { call: ToolCall; current: ToolJob | null }) {
  const tr = useTr()
  const [history, setHistory] = useState<ToolJob[]>([])
  const [image, setImage] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    try {
      setHistory(await call<ToolJob[]>('comfyJobs'))
    } catch (e) {
      setError(message(e))
    }
  }, [call])

  useEffect(() => {
    void load()
  }, [load, current?.id, current?.phase])

  useEffect(() => {
    if (current?.phase !== 'done') {
      setImage(null)
      return
    }
    let alive = true
    void call<{ image: string | null }>('comfyJobOutput')
      .then((reply) => alive && setImage(reply.image))
      .catch(() => {})
    return () => {
      alive = false
    }
  }, [call, current?.id, current?.phase])

  useEffect(() => {
    if (current?.phase !== 'queued' && current?.phase !== 'running') return
    const timer = setInterval(() => void load(), 2000)
    return () => clearInterval(timer)
  }, [current?.phase, load])

  const cancel = async () => {
    setError(null)
    try {
      await call('comfyCancel')
    } catch (e) {
      setError(message(e))
    }
  }

  const recent = history.filter((job) => job.id !== current?.id)
  return (
    <VStack gap={12}>
      {current ? <JobCard job={current} onCancel={() => void cancel()} /> : <Text size="caption" tone="secondary">{tr('No job yet: pick a template and run it.')}</Text>}
      {current?.phase === 'done' && image ? <img className="comfy-thumb" src={image} alt={tr('Last output')} /> : null}
      {recent.length > 0 ? (
        <CardSection title="Recent jobs">
          {recent.map((job) => (
            <HStack key={job.id} gap={8}>
              <Text tone="text" grow>{job.template ? tr(job.template) : tr('Workflow')}</Text>
              <Text size="caption" tone="secondary">{tr(PHASES[job.phase])}</Text>
              <Text size="caption" tone="secondary">{when(job.createdAt)}</Text>
            </HStack>
          ))}
        </CardSection>
      ) : null}
      {error ? <ErrorText text={error} /> : null}
    </VStack>
  )
}
