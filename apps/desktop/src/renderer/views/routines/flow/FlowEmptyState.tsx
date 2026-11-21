import { Button } from '../../../components/Controls'
import { Icon } from '../../../components/Icon'
import { VStack } from '../../../components/Stack'
import { Text } from '../../../components/Text'
import { useTr } from '../../../lib/use-language'

/**
 * The empty canvas and the empty list both show what a workflow looks like and one way to make it
 * (OpenBot nightly). One component, so the two views never drift apart.
 */
export function FlowEmptyState({ note, disabled, onNew }: { note?: string | null; disabled?: boolean; onNew: () => void }) {
  const tr = useTr()
  return (
    <VStack gap={10} align="center" className="outlined-card flow-state flow-empty">
      <Icon name="sparkles" size={28} />
      <Text size="callout" weight="semibold">{tr('No workflows yet')}</Text>
      <Text size="callout" tone="secondary">{note ?? tr('A workflow starts with a routine and hands its answer on to other bots.')}</Text>
      <div className="flow-sample" aria-hidden="true">
        <span className="flow-sample-node"><Icon name="clock.arrow.circlepath" size={16} />{tr('Routine')}</span>
        <span className="flow-sample-line" />
        <span className="flow-sample-node"><Icon name="bubble.left" size={16} />{tr('Bot')}</span>
        <span className="flow-sample-line" />
        <span className="flow-sample-node"><Icon name="photo" size={16} />{tr('Picture')}</span>
      </div>
      <Button disabled={disabled} onClick={onNew}>{tr('New workflow')}</Button>
    </VStack>
  )
}
