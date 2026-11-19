import { isWorking, threadName, type Bot, type ThreadInfo } from '@shared/models'
import { Icon } from '../components/Icon'
import { HStack, VStack } from '../components/Stack'
import { Text } from '../components/Text'
import { ThinkingOrb } from '../components/ThinkingOrb'
import { messagePreview } from '../lib/message-preview'
import { useModel } from '../lib/observable'
import { useTr } from '../lib/use-language'
import type { BotStore } from '../store/bot-store'
import { relativeTime } from './thread/chat-items'

/**
 * A bot's recent threads under its roster row: the same live list the chat's Threads tab shows,
 * indented and limited to a handful. Opening one is handled by the roster, over the chat.
 */
export function RosterThreads({ bot, store, threads, open, onOpen }: {
  bot: Bot
  store: BotStore
  threads: ThreadInfo[]
  open: boolean
  onOpen: (thread: ThreadInfo) => void
}) {
  const tr = useTr()
  useModel(store)
  // A side-thread turn writes to its own thread, which then sorts first. A reply newer than this
  // turn's start is the only working signal the store carries for a thread.
  const active = isWorking(bot) && bot.startedAt && threads[0] && threads[0].updatedAt >= bot.startedAt ? threads[0].id : null
  return (
    <div className={`roster-threads ${open ? 'open' : ''}`} aria-hidden={!open} inert={!open}>
      <div className="roster-threads-inner">
        {!threads.length ? <Text size="caption" tone="tertiary" className="roster-threads-empty">{tr('No threads yet.')}</Text> : null}
        {threads.slice(0, RECENT).map((thread) => {
          const unread = thread.unread > 0
          const working = active === thread.id
          const label = `${threadName(thread)}${unread ? `, ${thread.unread} unread` : ''}${working ? `, ${tr('Working')}` : ''}`
          return (
            <button key={thread.id} className="roster-thread-row press" aria-label={label}
              onClick={() => onOpen(thread)}>
              <VStack gap={1} shrink>
                <HStack gap={6} align="center" shrink>
                  {unread ? <span className="roster-thread-dot" aria-hidden /> : null}
                  <Text size="callout" weight={unread ? 'semibold' : 'medium'} truncate>{threadName(thread)}</Text>
                </HStack>
                <Text size="caption" tone="secondary" truncate>
                  {thread.lastText
                    ? `${thread.lastAuthor === 'user' ? tr('You') : (thread.lastAuthor ?? '')} · ${messagePreview(thread.lastText)}`
                    : relativeTime.day(thread.updatedAt)}
                </Text>
              </VStack>
              {working ? <span className="roster-thread-working" role="img" aria-label={tr('Working')} title={tr('Working')}><ThinkingOrb size={12} color="var(--secondary)" /></span> : null}
              {unread ? <span className="unread-badge">{thread.unread}</span> : null}
              <Icon name="chevron.right" size={9} className="roster-thread-open" />
            </button>
          )
        })}
      </div>
    </div>
  )
}

/** How many of a bot's threads fit under its row. */
const RECENT = 6
