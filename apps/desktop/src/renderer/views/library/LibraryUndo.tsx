import { Button, IconButton } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { HStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'

/** A Library move's confirmation, with the Undo the host change offers (the inverse move, run from here). */
export function LibraryUndo({ text, undo, dismiss }: { text: string; undo: () => void; dismiss: () => void }) {
  const tr = useTr()
  return (
    <HStack gap={8} align="center" className="library-undo" role="status">
      <Icon name="checkmark.circle.fill" size={14} color="var(--added)" />
      <Text size="callout" grow truncate>{text}</Text>
      <Button kind="secondary" onClick={undo}>Undo</Button>
      <IconButton title={tr('Dismiss')} icon="xmark" onClick={dismiss} />
    </HStack>
  )
}
