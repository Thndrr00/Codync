import { Button, Spinner } from '../../../components/Controls'
import { HStack, Spacer, VStack } from '../../../components/Stack'
import { Text } from '../../../components/Text'
import { ModalHeader, Sheet } from '../../../components/Overlay'
import { historyStatus, runSummary, type FlowStep, type WorkflowRunPage, type WorkflowStepSource } from '../../../lib/routine-flow.ts'
import { useTr } from '../../../lib/use-language'
import { StatusChip } from './FlowNodeCard'
import { StepResult } from './StepResult'

/** A portable history step in the shape the existing result chip already renders. */
function asFlowStep(step: WorkflowRunPage['runs'][number]['steps'][number]): FlowStep {
  const at = new Date(step.startedAt ?? step.endedAt ?? 0).getTime() || 0
  return {
    id: step.id,
    runId: step.runId ?? '',
    botId: step.nodeId,
    deliveryId: null,
    input: '',
    output: step.output,
    status: historyStatus(step.status),
    error: step.error,
    createdAt: at,
    updatedAt: at,
  }
}

/**
 * The numbered, paginated run history. Each attempt shows its branch, wait state and the ports it
 * read; a failed attempt can be retried and a running step or whole run can be stopped by id.
 */
export function RunHistorySheet({ open, onClose, page, loading, error, sourcesByStep, onLoadMore, onRetry, onStopStep, onStopRun, onShowSettings }: {
  open: boolean
  onClose: () => void
  page: WorkflowRunPage | null
  loading: boolean
  error: string | null
  sourcesByStep: Record<string, WorkflowStepSource[] | null>
  onLoadMore: () => void
  onRetry: (stepId: string) => void
  onStopStep: (stepId: string) => void
  onStopRun: (runId: string) => void
  onShowSettings: (stepId: string) => void
}) {
  const tr = useTr()
  return (
    <Sheet open={open} onClose={onClose} width={640} height={720}>
      <VStack gap={12} className="routine-flow-sheet flow-run-history">
        <ModalHeader icon="list.bullet" title={tr('Run history')} onClose={onClose} />
        {error ? <Text size="caption" tone="danger">{error}</Text> : null}
        {!page && loading ? <HStack gap={8}><Spinner /><Text size="callout" tone="secondary">{tr('Loading runs…')}</Text></HStack> : null}
        {page && page.runs.length === 0 && page.soloSteps.length === 0 ? (
          <Text size="callout" tone="secondary">{tr('No runs yet.')}</Text>
        ) : null}
        {(page?.runs ?? []).map((detail) => (
          <VStack key={detail.run.id} gap={8} className="flow-history-run">
            <HStack gap={8} align="center">
              <Text weight="semibold" grow>{runSummary(detail.run, detail.steps.length, tr)}</Text>
              <StatusChip status={historyStatus(detail.run.status)} />
              {detail.run.status === 'running' || detail.run.status === 'approval' || detail.run.status === 'stopping' ? (
                <Button kind="secondary" onClick={() => onStopRun(detail.run.id)}>{tr('Stop run')}</Button>
              ) : null}
            </HStack>
            {detail.run.error ? <Text size="caption" tone="danger">{detail.run.error}</Text> : null}
            {detail.steps.map((step) => (
              <VStack key={step.id} gap={4} className="flow-history-step">
                <HStack gap={8} align="center">
                  <Text size="callout" truncate grow>{step.nodeId}</Text>
                  <StatusChip status={historyStatus(step.status)} />
                  <Spacer />
                  {step.status === 'running' || step.status === 'approval' ? (
                    <Button kind="secondary" onClick={() => onStopStep(step.id)}>{tr('Stop step')}</Button>
                  ) : null}
                  {step.status === 'failed' ? (
                    <Button onClick={() => onRetry(step.id)}>{tr('Retry')}</Button>
                  ) : null}
                  <Button kind="secondary" onClick={() => onShowSettings(step.id)}>{tr('Settings')}</Button>
                </HStack>
                {step.error ? <Text size="caption" tone="danger" lines={3}>{step.error}</Text> : null}
                <StepResult
                  step={asFlowStep(step)}
                  compact
                  attempt={step.attempt}
                  branch={step.branch ?? undefined}
                  waiting={step.waiting}
                  sources={sourcesByStep[step.id] ?? []}
                />
              </VStack>
            ))}
          </VStack>
        ))}
        {(page?.soloSteps ?? []).map((step) => (
          <VStack key={step.id} gap={4} className="flow-history-step">
            <Text size="callout" truncate>{step.nodeId} · {tr('No run')}</Text>
            <StepResult step={asFlowStep(step)} compact attempt={step.attempt} branch={step.branch ?? undefined} waiting={step.waiting} />
          </VStack>
        ))}
        {page?.nextBefore ? (
          <Button kind="secondary" disabled={loading} onClick={onLoadMore}>{tr('Load older runs')}</Button>
        ) : null}
      </VStack>
    </Sheet>
  )
}
