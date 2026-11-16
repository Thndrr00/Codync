import { useState } from 'react'
import { isGroup, type Bot } from '@shared/models'
import { Button, CardSection, ValueRow } from '../../components/Controls'
import { Dialog, DialogField } from '../../components/Overlay'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'
import { errorText } from '../settings/parts'

/**
 * A chat's or group's history actions, from its settings: Delete history removes its messages and turns
 * only; New conversation keeps them on screen and starts the agents fresh; Compact context asks the
 * agent (or every member of a group) to summarize what it keeps. Ported from OpenBot.
 */
export function HistoryActions({ bot }: { bot: Bot }) {
  const tr = useTr()
  const store = useStore()
  const [confirming, setConfirming] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const run = async (action: () => Promise<unknown> | void) => {
    setError(null)
    try {
      await action()
    } catch (e) {
      setError(errorText(e))
    }
  }
  return (
    <CardSection
      title={tr('History')}
      footer={tr(
        'Deleting the history removes this chat\'s messages and turns. Its setup, memories, routines, artifacts and files stay.',
      )}
    >
      <ValueRow label={tr('Delete history')} detail={tr('Removes the messages and turns of this chat. Other chats and side threads keep theirs.')}>
        <Button kind="secondary" onClick={() => setConfirming(true)}>
          {tr('Delete history')}
        </Button>
      </ValueRow>
      <ValueRow
        label={tr('New conversation')}
        detail={tr('Keeps the messages here and starts the agents fresh.')}
      >
        <Button kind="secondary" onClick={() => void run(() => store.newSession(bot.id))}>
          {tr('New conversation')}
        </Button>
      </ValueRow>
      <ValueRow
        label={tr('Compact context')}
        detail={
          isGroup(bot)
            ? tr('Asks every member to summarize what it keeps. A member shares one conversation across its chats, so its own chat is compacted too.')
            : tr('Asks the agent to summarize the conversation it keeps, so the next turns send less context.')
        }
      >
        <Button kind="secondary" onClick={() => void run(() => store.compactContext(bot.id))}>
          {tr('Compact context')}
        </Button>
      </ValueRow>
      {error ? (
        <Text size="footnote" tone="danger">
          {error}
        </Text>
      ) : null}
      <DeleteHistoryDialog bot={bot} open={confirming} onClose={() => setConfirming(false)} />
    </CardSection>
  )
}

/** Typed confirmation: the person types the chat's name before its history goes. */
export function DeleteHistoryDialog({ bot, open, onClose }: { bot: Bot; open: boolean; onClose: () => void }) {
  const tr = useTr()
  const store = useStore()
  const [typed, setTyped] = useState('')
  const [error, setError] = useState<string | null>(null)
  const matches = typed.trim() === bot.name
  const remove = async () => {
    setError(null)
    try {
      await store.deleteHistory(bot.id)
    } catch (e) {
      setError(errorText(e))
    }
  }
  return (
    <Dialog
      open={open}
      title={tr('Delete {name}\'s history?').replace('{name}', bot.name)}
      message={tr(
        'Its messages and turns go. Its setup, memories, routines, artifacts and files stay, and no other chat changes.',
      )}
      actions={[{ title: 'Delete history', destructive: true, disabled: !matches, action: () => void remove() }]}
      onClose={() => {
        setTyped('')
        onClose()
      }}
    >
      <DialogField
        value={typed}
        onChange={setTyped}
        onSubmit={() => matches && void remove()}
        placeholder={bot.name}
        label={tr('Type {name} to confirm').replace('{name}', bot.name)}
      />
      {error ? (
        <Text size="footnote" tone="danger">
          {error}
        </Text>
      ) : null}
    </Dialog>
  )
}
