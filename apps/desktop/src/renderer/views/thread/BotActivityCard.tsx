import { useEffect, useMemo, useState } from 'react'
import { Button, ChoicePicker, IconButton, Spinner } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useStore } from '../../store/context'
import './activity.css'

interface ActivityItem {
  id: string
  kind: 'user' | 'agent' | 'permission' | 'notice'
  text: string
  at: number
  status?: string
  options?: { optionId: string; name?: string; kind?: string }[]
}

interface ActivityRun {
  id: string
  routineId: string
  status: string
  at: number | null
  detail?: string | null
}

interface Activity {
  items: ActivityItem[]
  runs: ActivityRun[]
}

const iconOf = (kind: ActivityItem['kind']) =>
  kind === 'user' ? 'person.crop.circle' : kind === 'agent' ? 'bubble.left' : kind === 'permission' ? 'hand.raised' : 'info.circle'

const toneOf = (kind: ActivityItem['kind'] | 'run') =>
  kind === 'permission' ? 'attention' : kind === 'run' ? 'secondary' : 'tertiary'

/**
 * A bot's recent activity, newest first: the messages and cards in its main chat and the runs of its
 * routines, without the tool steps and thinking of each turn (the chat's trace has those).
 */
export function BotActivityCard({ botId }: { botId: string }) {
  const store = useStore()
  const [data, setData] = useState<Activity | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)
  const [rev, setRev] = useState(0)
  const [filter, setFilter] = useState<'all' | 'messages' | 'approvals' | 'routines'>('all')
  const [answering, setAnswering] = useState<Record<string, boolean>>({})

  useEffect(() => {
    let stopped = false
    const load = async () => {
      setLoading(true)
      try {
        const result = await (await store.ready()).call<Activity>('botActivity', { botId, limit: 120 })
        if (stopped) return
        setData(result)
        setError(null)
      } catch (e) {
        if (!stopped && !store.isOffline) setError(e instanceof Error ? e.message : String(e))
      } finally {
        if (!stopped) setLoading(false)
      }
    }
    void load()
    const timer = setInterval(load, 15_000)
    return () => {
      stopped = true
      clearInterval(timer)
    }
  }, [botId, store, rev])

  const answer = async (id: string, optionId: string) => {
    setAnswering((current) => ({ ...current, [id]: true }))
    try {
      await store.answerCard(id, optionId)
      setRev((n) => n + 1)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setAnswering((current) => {
        const next = { ...current }
        delete next[id]
        return next
      })
    }
  }

  const rows = useMemo(() => {
    const items = (data?.items ?? []).map((item) => ({ ...item, row: 'item' as const }))
    const runs = (data?.runs ?? []).map((run) => ({ id: run.id, kind: 'run' as const, text: run.detail ?? `${run.status}`, at: run.at ?? 0, row: 'run' as const }))
    const all = [...items, ...runs].sort((a, b) => b.at - a.at).slice(0, 140)
    return all.filter((row) => {
      if (filter === 'all') return true
      if (filter === 'routines') return row.row === 'run'
      if (filter === 'approvals') return row.kind === 'permission'
      return row.kind === 'user' || row.kind === 'agent'
    })
  }, [data, filter])

  return (
    <VStack gap={8} className="activity-card">
      <HStack gap={8} align="center">
        <Text size="callout" weight="semibold" grow>Recent activity</Text>
        {loading ? <Spinner size={12} /> : null}
      </HStack>
      <ChoicePicker
        selection={filter}
        options={[
          { id: 'all', label: 'Everything' },
          { id: 'messages', label: 'Messages' },
          { id: 'approvals', label: 'Approvals' },
          { id: 'routines', label: 'Routines' },
        ]}
        onChange={setFilter}
      />
      {error ? <Text size="caption" tone="danger">{error}</Text> : null}
      {!loading && rows.length === 0 ? <Text size="caption" tone="secondary">Nothing yet. Messages and routine runs show here.</Text> : null}
      <div className="activity-scroll">
        {rows.map((row) => {
          const options = row.row === 'item' ? row.options : undefined
          const allow = options?.find((option) => option.kind === 'allow_once') ?? options?.find((option) => option.kind?.startsWith('allow'))
          const deny = options?.find((option) => option.kind === 'reject_once') ?? options?.find((option) => option.kind?.startsWith('reject'))
          return (
            <HStack key={`${row.row}:${row.id}`} gap={8} align="start" className="activity-row">
              <span className={`activity-icon ${toneOf(row.kind as ActivityItem['kind'])}`}>
                <Icon name={row.kind === 'run' ? 'arrow.clockwise' : iconOf(row.kind as ActivityItem['kind'])} size={12} />
              </span>
              <VStack gap={2} grow className="activity-text">
                <Text size="caption" tone="secondary">
                  {row.kind === 'user' ? 'You' : row.kind === 'agent' ? 'Bot' : row.kind === 'permission' ? 'Approval' : row.kind === 'run' ? 'Routine' : 'Notice'}
                  {row.at ? ` · ${new Date(row.at).toLocaleString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })}` : ''}
                </Text>
                <Text size="callout" className="activity-line">{row.text || '—'}</Text>
              </VStack>
              {row.row === 'item' && row.kind === 'permission' && row.status === 'pending' && options ? (
                answering[row.id] ? <Spinner size={11} /> : (
                  <HStack gap={2} className="activity-answer">
                    {deny ? <IconButton title="Deny" icon="xmark" size={24} onClick={() => void answer(row.id, deny.optionId)} /> : null}
                    {allow ? <IconButton title="Allow" icon="checkmark" size={24} onClick={() => void answer(row.id, allow.optionId)} /> : null}
                  </HStack>
                )
              ) : null}
            </HStack>
          )
        })}
      </div>
      <Button kind="secondary" disabled={loading} onClick={() => setRev((n) => n + 1)}>Refresh</Button>
    </VStack>
  )
}
