import { useEffect, useState } from 'react'
import type { Bot } from '@shared/models'
import { Button, CardSection, ChoicePicker, ValueRow } from '../../components/Controls'
import { HStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import type { MaintenanceStore } from '../../store/maintenance-store'
import { Field } from '../marketplace/pieces'

const FLEET = '*'

/** Daily token limits, for all bots together or for one: a warning at 80%, and routines pause at the limit. */
export function TokenBudgetSection({ store, bots, online }: { store: MaintenanceStore; bots: Bot[]; online: boolean }) {
  const [scope, setScope] = useState(FLEET)
  const [limit, setLimit] = useState('')
  useEffect(() => { if (online) void store.loadBudgets() }, [store, online])
  const name = (id: string) => (id === FLEET ? 'All bots' : bots.find((b) => b.id === id)?.name ?? 'A deleted bot')
  const tokens = Number.parseInt(limit.replace(/[\s,]/g, ''), 10)
  const valid = Number.isFinite(tokens) && tokens > 0
  return (
    <CardSection title="Daily token limits"
      footer="Counts the tokens each turn used today (not cached reads). At 80% the bot's chat says so; at the limit its routines pause until you turn them back on. Your own messages are never refused.">
      {store.budgets.map((b) => (
        <ValueRow key={b.botId} label={name(b.botId)} detail={`${b.used.toLocaleString()} of ${b.limit.toLocaleString()} today`}>
          <HStack gap={8}>
            {b.level !== 'within' ? <Text tone={b.level === 'exceeded' ? 'danger' : 'attention'}>{b.level === 'exceeded' ? 'Limit reached' : '80% used'}</Text> : null}
            <Button kind="secondary" disabled={!online || store.busy} onClick={() => void store.setBudget(b.botId, null)}>Remove</Button>
          </HStack>
        </ValueRow>
      ))}
      {!store.budgets.length ? <Text tone="secondary">No limits set.</Text> : null}
      <ValueRow label="Set a limit for">
        <ChoicePicker selection={scope} options={[{ id: FLEET, label: 'All bots' }, ...bots.map((b) => ({ id: b.id, label: b.name }))]} onChange={setScope} />
      </ValueRow>
      <Field label="Tokens per day" value={limit} onChange={setLimit} placeholder="For example 5,000,000" />
      <Button disabled={!online || store.busy || !valid} onClick={() => void store.setBudget(scope, tokens)}>Save limit</Button>
    </CardSection>
  )
}
