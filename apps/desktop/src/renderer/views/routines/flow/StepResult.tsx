import { useEffect, useState } from 'react'
import { isVideoName } from '@shared/models'
import { Spinner } from '../../../components/Controls'
import { Icon } from '../../../components/Icon'
import { HStack, VStack } from '../../../components/Stack'
import { Text } from '../../../components/Text'
import type { FlowStep } from '../../../lib/routine-flow.ts'
import { useTr } from '../../../lib/use-language'

/**
 * The newest file of a step's result, shown as soon as the run ends. Round thumbnails walk the
 * files of the same step; a new run (a new step id) starts on the newest one again, never on a stale
 * file the person happened to click before. A step still working shows a spinner instead of an
 * empty result, so "making it" never reads as "nothing came back".
 */
export function StepResult({ step, compact = false }: { step: FlowStep; compact?: boolean }) {
  const tr = useTr()
  const files = step.files ?? []
  const [index, setIndex] = useState(files.length - 1)
  useEffect(() => setIndex(files.length - 1), [step.id, files.length])
  if (files.length === 0) {
    if (step.status === 'running') {
      return (
        <HStack gap={6} align="center" className="flow-step-result">
          <Spinner size={12} />
          <Text size="caption" tone="secondary">{tr('Making the result…')}</Text>
        </HStack>
      )
    }
    return step.output ? <Text size="caption" tone="secondary" lines={4}>{step.output}</Text> : null
  }
  const at = Math.min(Math.max(index, 0), files.length - 1)
  const file = files[at]
  return (
    <VStack gap={6} className="flow-step-result" data-status={step.status}>
      <Text size="caption" tone="secondary" weight="semibold">{tr('Result')}</Text>
      <HStack gap={8} align="center" className="flow-result-switcher">
        <button type="button" className="flow-result-arrow" aria-label={tr('Older result')} title={tr('Older result')} disabled={at <= 0} onClick={() => setIndex(at - 1)}>
          <Icon name="chevron.left" size={12} weight="bold" />
        </button>
        <HStack gap={6} align="center" className="flow-result-thumbs">
          {files.map((item, i) => (
            <button
              key={item.id}
              type="button"
              className="flow-result-thumb"
              data-active={i === at}
              title={item.name}
              aria-label={item.name}
              aria-pressed={i === at}
              onClick={() => setIndex(i)}
            >
              <Icon name={isVideoName(item.name) ? 'film' : 'photo'} size={16} />
            </button>
          ))}
        </HStack>
        <button type="button" className="flow-result-arrow" aria-label={tr('Newer result')} title={tr('Newer result')} disabled={at >= files.length - 1} onClick={() => setIndex(at + 1)}>
          <Icon name="chevron.right" size={12} weight="bold" />
        </button>
        {step.status === 'running' ? <Spinner size={12} /> : null}
      </HStack>
      <Text size="callout" grow truncate title={file.name}>{file.name}</Text>
      {compact ? null : (
        <Text size="caption" tone="secondary">
          {tr('{index} of {total} files').replace('{index}', String(at + 1)).replace('{total}', String(files.length))}
        </Text>
      )}
    </VStack>
  )
}
