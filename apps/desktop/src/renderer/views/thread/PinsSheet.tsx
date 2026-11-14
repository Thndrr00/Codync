import { useCallback, useEffect, useState } from 'react'
import { type PinnedMessage, type Pinner } from '@shared/pins'
import { Icon } from '../../components/Icon'
import { IconButton, Spinner } from '../../components/Controls'
import { ModalHeader, SheetPage } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'
import './pins.css'

/** Who pinned a message, as the list says it. */
function pinner(tr: ReturnType<typeof useTr>, pinner: Pinner, agent: string) {
  if (pinner.kind === 'agent') return tr('{name} (agent)').replace('{name}', agent)
  return pinner.device ?? tr('You')
}

/**
 * The chat's pinned messages, newest first, across the chat and its threads. A pin opens its thread;
 * the agent's pins are taken back here like its own tool takes them back.
 */
export function PinsSheet({ botId, close, openThread }: { botId: string; close: () => void; openThread: (root: string) => void }) {
  const store = useStore()
  const tr = useTr()
  const [pins, setPins] = useState<PinnedMessage[] | null>(null)
  const [error, setError] = useState<string | null>(null)
  const bot = store.bots.get(botId)
  const agent = bot?.name ?? tr('The agent')

  const load = useCallback(async () => {
    try {
      setPins((await store.pinnedMessages(botId)).pins)
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason))
    }
  }, [botId, store])

  useEffect(() => {
    void load()
  }, [load])

  const unpin = async (pin: PinnedMessage) => {
    setPins((list) => list?.filter((one) => one.messageId !== pin.messageId) ?? null)
    try {
      await store.setPinnedMessage({ id: pin.messageId, botId }, false)
      await load()
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason))
    }
  }

  return (
    <SheetPage>
      <ModalHeader
        icon="pin"
        title={tr('Pinned messages')}
        subtitle={tr('Kept at hand in this chat. A message can be unpinned by whoever reads it here.')}
        onClose={close}
      />
      <div className="pins-hairline" />
      {pins === null && !error ? (
        <HStack className="pins-loading" justify="center">
          <Spinner size={16} />
        </HStack>
      ) : null}
      {error ? <Text role="alert" tone="danger" className="pins-error">{error}</Text> : null}
      {pins?.length === 0 && !error ? (
        <VStack className="pins-empty" gap={4}>
          <Icon name="pin" size={22} color="var(--secondary)" />
          <Text tone="secondary">{tr("Nothing is pinned here yet. Open a message's menu and pick Pin.")}</Text>
        </VStack>
      ) : null}
      <div className="pins-scroll">
        {pins?.map((pin) => (
          <div key={pin.messageId} className="pin-row">
            <button
              className="pin-main press"
              onClick={() => {
                if (pin.threadId) openThread(pin.threadId)
                close()
              }}
            >
              {pin.threadTitle ? <Text size="caption" tone="tertiary" truncate>{pin.threadTitle}</Text> : null}
              <Text size="body" truncate>{pin.text || tr('A message without text')}</Text>
              <Text size="caption" tone="tertiary" truncate>
                {tr('{who} · {author}').replace('{who}', pinner(tr, pin.pinnedBy, agent)).replace('{author}', pin.author === 'agent' ? agent : bot?.name ?? tr('You'))}
              </Text>
            </button>
            <IconButton title={tr('Unpin')} icon="pin.slash" size={28} onClick={() => void unpin(pin)} />
          </div>
        ))}
      </div>
    </SheetPage>
  )
}
