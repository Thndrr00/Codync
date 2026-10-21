import { useLayoutEffect, useRef } from 'react'
import { useStore } from '../store/context'
import { Sheet, ModalHeader } from '../components/Overlay'
import { Text } from '../components/Text'
import { VStack } from '../components/Stack'
import { Button } from '../components/Controls'

/** A bounded lane window stays separate from the mirror so it cannot create a history paging gap. */
export function MessageContext({ botId, openThread }: { botId: string; openThread: (root: string) => void }) {
  const store = useStore()
  const context = store.messageContext
  const target = useRef<HTMLDivElement>(null)
  useLayoutEffect(() => { target.current?.scrollIntoView({ block: 'center' }) }, [context?.id])
  if (!context || context.botId !== botId) return null
  return <Sheet open onClose={() => store.closeMessageContext()} width={640} height={560} dismissOnDim>
    <ModalHeader title={store.bots.get(context.botId)?.name ?? 'Message'} subtitle={context.threadId ? 'Thread · search context' : 'Main chat · search context'} onClose={() => store.closeMessageContext()} />
    <VStack gap={16} className="roster-history">
      <Button kind="secondary" onClick={() => { if (context.threadId) openThread(context.threadId); store.closeMessageContext() }}>{context.threadId ? 'Continue in thread' : 'Continue in chat'}</Button>
      {context.entries.filter((e) => (e.threadId ?? null) === context.threadId && (e.kind === 'user' || e.kind === 'agent' && e.data.final)).map((entry) =>
        <div key={entry.id} ref={entry.id === context.id ? target : undefined} className="roster-hit" aria-current={entry.id === context.id || undefined}>
          <Text as="div" size="caption" tone="secondary">{entry.kind === 'user' ? 'You' : store.bots.get(context.botId)?.name}</Text>
          <Text as="div">{entry.data.text}</Text>
        </div>)}
    </VStack>
  </Sheet>
}
