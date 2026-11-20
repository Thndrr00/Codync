import { Button } from '../../components/Controls'
import { VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'

/**
 * The Library's three states, in the same outlined card as its rows: a short explanation with
 * one action when empty or failed, and a few shaped placeholder rows while loading (no spinner).
 * Illustration-free, and built only from the shared Text/Button and the theme's tokens.
 */
export function LibraryState({ kind, title, detail, action, onAction }: {
  kind: 'empty' | 'loading' | 'error'
  title: string
  detail?: string
  action?: string
  onAction?: () => void
}) {
  const tr = useTr()
  if (kind === 'loading') {
    return (
      <VStack gap={8} className="outlined-card library-state" aria-busy="true" aria-label={tr(title)}>
        {[0, 1, 2].map((i) => (
          <div key={i} className="library-skeleton-row">
            <span className="library-skeleton-icon" />
            <VStack gap={6}>
              <span className="library-skeleton-line" />
              <span className="library-skeleton-line short" />
            </VStack>
          </div>
        ))}
      </VStack>
    )
  }
  return (
    <VStack gap={4} className="outlined-card library-state">
      <Text size="callout" weight="semibold" tone={kind === 'error' ? 'danger' : 'text'} role={kind === 'error' ? 'alert' : undefined}>{tr(title)}</Text>
      {detail ? <Text size="callout" tone="secondary">{tr(detail)}</Text> : null}
      {action && onAction ? <Button onClick={onAction}>{tr(action)}</Button> : null}
    </VStack>
  )
}
