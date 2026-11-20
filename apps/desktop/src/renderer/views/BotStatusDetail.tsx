import { isWorking, needsInput, type Bot, type Entry } from '@shared/models'
import { Icon } from '../components/Icon'
import { HStack, VStack } from '../components/Stack'
import { Text } from '../components/Text'
import { useTr } from '../lib/use-language'
import type { BotStore } from '../store/bot-store'
import { relativeTime } from './thread/chat-items'
import './bot-status-detail.css'

/**
 * The Status panel in a bot's roster menu. Everything here is data the host already sends in the
 * mirror — the bot's live status, the turn's `startedAt`, tool entries in the chat and its model —
 * so it reads the store directly and never polls.
 */
export function BotStatusDetail({ bot, store }: { bot: Bot; store: BotStore }) {
  const tr = useTr()
  const working = isWorking(bot)
  const status = needsInput(bot) ? tr('Needs input') : working ? tr('Working') : bot.status === 'error' ? tr('Error') : tr('Idle')
  const tools = latestTools(store.chatEntries(bot.id))

  return (
    <VStack gap={10} className="bot-status-detail">
      <HStack gap={8} align="center">
        <Icon name={working ? 'bolt' : 'circle'} size={14} color={working ? 'var(--accent)' : 'var(--secondary)'} />
        <Text size="body" weight="semibold" tone="text" grow>{tr('Status')}</Text>
        <Text size="callout" tone={working ? 'accent' : 'secondary'}>{status}</Text>
      </HStack>
      {bot.activity ? <Text size="callout" tone="secondary">{bot.activity}</Text> : null}
      {bot.startedAt && working ? (
        <Row icon="clock" label={tr('Turn started')} value={relativeTime.separator(bot.startedAt, tr)} />
      ) : null}
      <Row icon={toolIcon(tools[0]?.data.toolKind)} label={tr('Tools run')}
        value={tools.length ? tools.map((entry) => entry.data.title ?? tr('Tool')).join(', ') : tr('None yet')} />
      {bot.model ? <Row icon="cpu" label={tr('Model')} value={bot.model} /> : null}
    </VStack>
  )
}

function Row({ icon, label, value }: { icon: string; label: string; value: string }) {
  return (
    <HStack gap={8} align="center">
      <Icon name={icon} size={12} color="var(--secondary)" />
      <Text size="callout" tone="secondary">{label}</Text>
      <Text size="callout" tone="text" grow truncate title={value}>{value}</Text>
    </HStack>
  )
}

/** The tools of the latest turn, most recent first, limited so the panel stays small. */
function latestTools(entries: Entry[]): Entry[] {
  return entries
    .filter((entry) => entry.kind === 'tool' && entry.data.bookkeeping !== true)
    .slice(-MAX_TOOLS)
    .reverse()
}

const toolIcon = (kind?: string) =>
  ({ read: 'doc.text', edit: 'pencil', delete: 'trash', move: 'arrow.right.doc.on.clipboard', search: 'magnifyingglass', execute: 'terminal', think: 'brain', fetch: 'globe' })[kind ?? ''] ?? 'wrench.and.screwdriver'

const MAX_TOOLS = 4

/** The menu row title, kept beside the panel it opens. */
export const BOT_STATUS_DETAIL_TITLE = 'Status'
