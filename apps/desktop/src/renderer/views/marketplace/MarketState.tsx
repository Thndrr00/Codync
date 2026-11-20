import { Button } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { SkeletonGrid } from './pieces'
import './market-state.css'

/**
 * The Extension panels' states: illustration-free cards with one explanation and one primary
 * action, or shaped placeholder rows while a catalog loads (the shared SkeletonGrid, no spinner).
 * Only shared Text/Button and theme tokens are used.
 */
export function MarketState({ kind, title, detail, action, onAction }: {
  kind: 'empty' | 'error'
  title: string
  detail?: string
  action?: string
  onAction?: () => void
}) {
  const tr = useTr()
  return (
    <VStack gap={6} className={`market-state ${kind}`} role={kind === 'error' ? 'alert' : undefined}>
      <span className="market-state-icon" aria-hidden>
        <Icon name={kind === 'error' ? 'exclamationmark.triangle' : 'tray'} size={20} />
      </span>
      <Text size="headline" weight="semibold" tone="text">{tr(title)}</Text>
      {detail ? <Text size="callout" tone="secondary">{tr(detail)}</Text> : null}
      {action && onAction ? <Button onClick={onAction}>{tr(action)}</Button> : null}
    </VStack>
  )
}

/** The shared skeleton rows, used while a catalog's first page is on its way. */
export function MarketLoading() {
  return (
    <VStack className="market-loading">
      <SkeletonGrid />
    </VStack>
  )
}
