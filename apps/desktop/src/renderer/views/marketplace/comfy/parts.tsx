/** Shared bits of ComfyUI's card tabs: the host call signature and the sentence for one preflight problem. */
import type { TemplatePreflightProblem } from '@shared/tools'
import { Button } from '../../../components/Controls'
import { HStack } from '../../../components/Stack'
import { Text } from '../../../components/Text'
import { useTr } from '../../../lib/use-language'

/** `useTool`'s call: a host method with the tool's id already added. */
export type ToolCall = <T = unknown>(method: string, body?: object, timeoutMs?: number) => Promise<T>

/** What a locale renders for a size in bytes. */
export function megabytes(bytes: number): string {
  return `${Math.round(bytes / 1024 / 1024)} MB`
}

/** A preflight problem with the one button that fixes it. */
export function ProblemRow({ problem, onAction }: { problem: TemplatePreflightProblem; onAction: (kind: string, problem: TemplatePreflightProblem) => void }) {
  const tr = useTr()
  const action = problem.action?.kind
  const label = action === 'download' ? 'Download' : action === 'start' ? 'Start' : action === 'installNode' ? 'Open ComfyUI' : null
  return (
    <HStack gap={8} align="start">
      <Text size="caption" tone={problem.action?.kind === 'useCpu' ? 'secondary' : 'attention'} grow>{tr(problem.message)}</Text>
      {label ? <Button kind="secondary" onClick={() => onAction(problem.action.kind, problem)}>{tr(label)}</Button> : null}
    </HStack>
  )
}
