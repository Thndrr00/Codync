import type { Entry } from '@shared/models'
import { IconButton, Spinner } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { HStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'

/**
 * A routine's short marker in the chat (or a thread). Where the host can open the routine it is a
 * button; in a thread sheet with no inspector beside it the same marker is plain text. A failed run
 * that still offers a retry can be tried again or ended there; the messages stay.
 */
export function RoutineNotice({ entry, open }: { entry: Entry; open?: () => void }) {
  const tr = useTr()
  const store = useStore()
  const answering = store.answering.get(entry.id)
  const failed = entry.data.source === 'scheduled_task' && entry.data.style !== undefined
  const label = tr(entry.data.text ?? 'Routine')
  return (
    <HStack gap={2} justify="center" align="center">
      {open ? (
        <button className="chat-routine-notice" onClick={open}>
          <Icon name="clock.arrow.circlepath" size={10} />
          {label}
        </button>
      ) : (
        <span className="chat-routine-notice">
          <Icon name="clock.arrow.circlepath" size={10} />
          {label}
        </span>
      )}
      {entry.data.retry &&
        (answering ? (
          <Spinner size={10} />
        ) : (
          <IconButton title={tr('Try again')} icon="arrow.clockwise" size={20} onClick={() => store.retryRoutineRun(entry)} />
        ))}
      {failed && !entry.data.cancelled && (
        <IconButton title={tr('Cancel task')} icon="xmark" size={20} onClick={() => store.cancelRoutineRun(entry)} />
      )}
      {entry.data.cancelled ? <Text size="caption" tone="tertiary">{tr('Cancelled')}</Text> : null}
    </HStack>
  )
}
