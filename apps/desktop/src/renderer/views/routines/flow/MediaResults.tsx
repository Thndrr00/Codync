import { Button, Spinner } from '../../../components/Controls'
import { HStack, VStack } from '../../../components/Stack'
import { Text } from '../../../components/Text'
import { groupMediaResults, mediaRetryLabel, type FlowStep } from '../../../lib/routine-flow.ts'
import { useTr } from '../../../lib/use-language'
import { StepResult } from './StepResult'

/**
 * The result area under a media step, for up to four results made at once. A pending result shows
 * the host's progress with the shared spinner, a failed one shows the host's own error and a priced
 * Retry that runs this step again, and the finished files share one StepResult thumbnails row, so
 * prev/next walks every result instead of only the newest step's file.
 */
export function MediaResults({ steps, price, busy, onRetry }: {
  /** The newest steps of this node, oldest first (older hosts have just one). */
  steps: readonly FlowStep[]
  /** Credits for one result, when the host sends a price. */
  price: number | null
  busy: boolean
  onRetry: () => void
}) {
  const tr = useTr()
  if (steps.length === 0) return null
  const { pending, failed, files } = groupMediaResults(steps, price)
  const newest = steps[steps.length - 1]
  return (
    <VStack gap={6} className="flow-media-results">
      {pending.map((take) => (
        <HStack gap={6} align="center" key={take.id}>
          <Spinner size={12} />
          <Text size="caption" tone="secondary">{tr('Making the result…')}</Text>
        </HStack>
      ))}
      {failed.map((take) => (
        <VStack gap={4} key={take.id}>
          {take.error ? <Text size="caption" tone="danger" lines={4}>{take.error}</Text> : null}
          <Button kind="secondary" disabled={busy} onClick={onRetry}>
            {mediaRetryLabel(take.credits, tr)}
          </Button>
        </VStack>
      ))}
      {files.length > 0 ? (
        <StepResult
          key={newest.id}
          step={{ ...newest, status: 'succeeded', output: null, error: null, files, results: undefined }}
        />
      ) : null}
    </VStack>
  )
}
