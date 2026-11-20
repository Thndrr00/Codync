import { useEffect, useState } from 'react'
import { IconButton } from '../../../components/Controls'
import { HStack, VStack } from '../../../components/Stack'
import { Text } from '../../../components/Text'
import type { FlowStep } from '../../../lib/routine-flow.ts'
import { useTr } from '../../../lib/use-language'

/** The newest file of a step's result, shown as soon as the run ends. Arrows walk the older files
 *  of the same step; a new run (a new step id) starts on the newest one again, never on a stale
 *  file the person happened to click before. */

export function StepResult({ step, compact = false }: { step: FlowStep; compact?: boolean }) {
  const tr = useTr()
  const files = step.files ?? []
  const [index, setIndex] = useState(files.length - 1)
  useEffect(() => setIndex(files.length - 1), [step.id, files.length])
  if (files.length === 0) {
    return step.output ? <Text size="caption" tone="secondary" lines={4}>{step.output}</Text> : null
  }
  const at = Math.min(Math.max(index, 0), files.length - 1)
  const file = files[at]
  return (
    <VStack gap={4} className="flow-step-result">
      <Text size="caption" tone="secondary" weight="semibold">{tr('Result')}</Text>
      <HStack gap={6} align="center">
        <IconButton title={tr('Older result')} icon="chevron.left" disabled={at <= 0} onClick={() => setIndex(at - 1)} />
        <Text size="callout" grow truncate title={file.name}>{file.name}</Text>
        <IconButton title={tr('Newer result')} icon="chevron.right" disabled={at >= files.length - 1} onClick={() => setIndex(at + 1)} />
      </HStack>
      {compact ? null : (
        <Text size="caption" tone="secondary">
          {tr('{index} of {total} files').replace('{index}', String(at + 1)).replace('{total}', String(files.length))}
        </Text>
      )}
    </VStack>
  )
}
