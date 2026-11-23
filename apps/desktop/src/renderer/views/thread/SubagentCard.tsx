import { BotAvatar } from '../../components/Avatar'
import { CardSection } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { HStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { subagentRows, type SubagentStatus } from '../../lib/bot-subagents.ts'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'
import { peerOf, type BotExchange } from './bot-exchange.ts'
import './subagent-card.css'

const STATUS_LABEL: Record<SubagentStatus, string> = {
  running: 'Running',
  completed: 'Completed',
  failed: 'Failed',
}

/**
 * A thread's subagents as one card: who the agent asked, and whether each one is still running,
 * done or failed. Clicking a row opens that teammate's work (OpenBot nightly triage-13639). The
 * statuses come from the host's exchange notices; nothing is invented here.
 */
export function SubagentCard({ exchanges, open }: { exchanges: BotExchange[]; open?: (peerId: string) => void }) {
  const tr = useTr()
  const store = useStore()
  const rows = subagentRows(exchanges.map((x) => ({
    id: x.id,
    peerId: peerOf(x),
    peerName: x.message.peerName,
    outcome: x.outcome.kind,
    request: x.message.text,
  })))
  if (!rows.length) return null
  return (
    <div className="subagent-card">
      <CardSection title={tr('Subagents')}>
        {rows.map((row) => {
          const bot = store.bots.get(row.peerId)
          const name = store.authorName(row.peerId, row.peerName)
          const status = tr(STATUS_LABEL[row.status])
          const body = (
            <>
              {bot ? <BotAvatar bot={bot} size={18} animated={false} /> : null}
              <HStack as="span" gap={6} grow shrink>
                <Text truncate weight="semibold">{name}</Text>
                {row.count > 1 ? <Text size="caption" tone="tertiary">×{row.count}</Text> : null}
              </HStack>
              <Text size="caption" tone={row.status === 'failed' ? 'danger' : row.status === 'running' ? 'accent' : 'secondary'}>{status}</Text>
              {open ? <Icon name="chevron.right" size={10} color="var(--tertiary)" /> : null}
            </>
          )
          return open ? (
            <button key={row.id} className="press subagent-row" title={row.request} aria-label={`${name}, ${status}`} onClick={() => open(row.peerId)}>
              {body}
            </button>
          ) : (
            <HStack key={row.id} gap={6} align="center" className="subagent-row static" title={row.request} aria-label={`${name}, ${status}`}>
              {body}
            </HStack>
          )
        })}
      </CardSection>
    </div>
  )
}
