import { useEffect, useState } from 'react'
import { isGroup, type Bot } from '@shared/models'
import { BotAvatar } from '../../components/Avatar'
import { Button, IconButton, Spinner } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { HStack, Spacer, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import type { BotStore } from '../../store/bot-store'
import './inbox.css'

interface ApprovalOption {
  optionId: string
  name?: string
  kind?: string
}

interface Approval {
  id: string
  title: string
  detail: string
  since: number
  options?: ApprovalOption[]
}

interface Run {
  id: string
  routineId: string
  status: string
  startedAt: number
  finishedAt?: number | null
  detail?: string | null
}

interface Thread {
  id: string
  unread: number
}

interface Tasks {
  approvals: Approval[]
  runs: Run[]
  threads: Thread[]
}

interface BotInbox {
  bot: Bot
  store: BotStore
  approvals: Approval[]
  threads: Thread[]
  runs: Run[]
}

const needsAttention = (run: Run) => run.status === 'failed' || run.status === 'recovering'

/** Everything across this window's bots that waits on a person: approvals, unread threads, failed runs. */
function useInbox(stores: BotStore[]): { boxes: BotInbox[]; loading: boolean; error: string | null; reload: () => void } {
  const [boxes, setBoxes] = useState<BotInbox[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [rev, setRev] = useState(0)
  // The window rebuilds this list on every render; only the set of computers should reload it.
  const storeKey = stores.map((s) => s.computer.id).join('|')

  useEffect(() => {
    let stopped = false
    const load = async () => {
      setLoading(true)
      try {
        const results = await Promise.all(
          stores.map(async (store) => {
            if (store.connection.kind !== 'online' || store.mismatch) return []
            const client = await store.ready()
            const bots = [...store.bots.values()].filter((bot) => !bot.hidden && !isGroup(bot))
            const rows = await Promise.all(
              bots.map(async (bot): Promise<BotInbox | null> => {
                const tasks = await client.call<Tasks>('botTasks', { botId: bot.id })
                const approvals = tasks.approvals ?? []
                const threads = tasks.threads ?? []
                const runs = (tasks.runs ?? []).filter(needsAttention)
                if (approvals.length === 0 && threads.length === 0 && runs.length === 0) return null
                return { bot, store, approvals, threads, runs }
              }),
            )
            return rows.filter((row): row is BotInbox => row !== null)
          }),
        )
        if (stopped) return
        const flat = results.flat().sort((a, b) => {
          const aAt = a.approvals[0]?.since ?? a.runs[0]?.startedAt ?? 0
          const bAt = b.approvals[0]?.since ?? b.runs[0]?.startedAt ?? 0
          return bAt - aAt
        })
        setBoxes(flat)
        setError(null)
      } catch (e) {
        if (!stopped) setError(e instanceof Error ? e.message : String(e))
      } finally {
        if (!stopped) setLoading(false)
      }
    }
    void load()
    const timer = setInterval(load, 8000)
    return () => {
      stopped = true
      clearInterval(timer)
    }
  }, [storeKey, rev])

  return { boxes, loading, error, reload: () => setRev((n) => n + 1) }
}

const when = (ms: number) => new Date(ms).toLocaleString(undefined, { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' })

/** One place for the approvals, unread threads and failed runs that wait across every bot. */
export function InboxView({ stores, onOpen }: { stores: BotStore[]; onOpen: (computerId: string, botId: string) => void }) {
  const tr = useTr()
  const { boxes, loading, error, reload } = useInbox(stores)
  const [answering, setAnswering] = useState<Record<string, boolean>>({})
  const [answerError, setAnswerError] = useState<string | null>(null)
  const total = boxes.reduce((n, b) => n + b.approvals.length + b.threads.length + b.runs.length, 0)
  const answer = async (box: BotInbox, approval: Approval, optionId: string) => {
    setAnswering((current) => ({ ...current, [approval.id]: true }))
    setAnswerError(null)
    try {
      await box.store.answerCard(approval.id, optionId)
      reload()
    } catch (e) {
      setAnswerError(e instanceof Error ? e.message : String(e))
    } finally {
      setAnswering((current) => {
        const next = { ...current }
        delete next[approval.id]
        return next
      })
    }
  }

  return (
    <div className="inbox-page">
      <header className="inbox-header">
        <VStack gap={2}>
          <Text size="title3" weight="semibold">{tr('Inbox')}</Text>
          <Text size="caption" tone="secondary">
            {total === 0 ? tr('Nothing waits on you.') : `${total} ${tr('things wait on you')}`}
          </Text>
        </VStack>
        <Spacer />
        {loading ? <Spinner /> : null}
        <IconButton title={tr('Refresh')} icon="arrow.clockwise" onClick={reload} />
      </header>

      {error ? <Text size="caption" tone="danger">{error}</Text> : null}
      {answerError ? <Text size="caption" tone="danger">{answerError}</Text> : null}

      {!loading && boxes.length === 0 ? (
        <div className="inbox-empty">
          <Icon name="checkmark.circle" size={34} />
          <Text size="body" weight="medium">{tr('All clear')}</Text>
          <Text size="caption" tone="secondary">
            {tr('Approvals, unread replies and failed runs from every bot land here.')}
          </Text>
        </div>
      ) : null}

      <div className="inbox-list">
        {boxes.map((box) => (
          <section key={`${box.store.computer.id}:${box.bot.id}`} className="inbox-bot">
            <button type="button" className="inbox-bot-head" onClick={() => onOpen(box.store.computer.id, box.bot.id)}>
              <BotAvatar bot={box.bot} size={34} animated={false} />
              <span className="inbox-bot-name">{box.bot.name}</span>
              <Spacer />
              {box.approvals.length ? <span className="inbox-count attention">{box.approvals.length}</span> : null}
              {box.threads.length ? <span className="inbox-count">{box.threads.reduce((n, t) => n + t.unread, 0)}</span> : null}
              {box.runs.length ? <span className="inbox-count danger">{box.runs.length}</span> : null}
              <Icon name="chevron.right" size={11} />
            </button>

            {box.approvals.map((approval) => {
              const allow = approval.options?.find((option) => option.kind === 'allow_once') ?? approval.options?.find((option) => option.kind?.startsWith('allow'))
              const deny = approval.options?.find((option) => option.kind === 'reject_once') ?? approval.options?.find((option) => option.kind?.startsWith('reject'))
              return (
                <div key={approval.id} className="inbox-row inbox-approval">
                  <button type="button" className="inbox-approval-open" onClick={() => onOpen(box.store.computer.id, box.bot.id)}>
                    <span className="inbox-row-icon attention"><Icon name="hand.raised" size={13} /></span>
                    <span className="inbox-row-text">
                      <span className="inbox-row-title">{approval.title}</span>
                      <span className="inbox-row-sub">{approval.detail || tr('Waiting for your approval')}</span>
                    </span>
                    <time>{when(approval.since)}</time>
                  </button>
                  {answering[approval.id] ? <Spinner size={12} /> : (
                    <HStack gap={2} className="inbox-approval-actions">
                      {deny ? <IconButton title={tr('Deny')} icon="xmark" size={26} onClick={() => void answer(box, approval, deny.optionId)} /> : null}
                      {allow ? <IconButton title={tr('Allow')} icon="checkmark" size={26} onClick={() => void answer(box, approval, allow.optionId)} /> : null}
                    </HStack>
                  )}
                </div>
              )
            })}

            {box.threads.map((thread) => (
              <button key={thread.id} type="button" className="inbox-row" onClick={() => onOpen(box.store.computer.id, box.bot.id)}>
                <span className="inbox-row-icon"><Icon name="bubble.left" size={13} /></span>
                <span className="inbox-row-text">
                  <span className="inbox-row-title">{tr('New replies in a thread')}</span>
                  <span className="inbox-row-sub">{thread.unread} {tr(thread.unread === 1 ? 'unread message' : 'unread messages')}</span>
                </span>
                <Icon name="chevron.right" size={11} />
              </button>
            ))}

            {box.runs.map((run) => (
              <button key={run.id} type="button" className="inbox-row" onClick={() => onOpen(box.store.computer.id, box.bot.id)}>
                <span className="inbox-row-icon danger"><Icon name="exclamationmark.triangle" size={13} /></span>
                <span className="inbox-row-text">
                  <span className="inbox-row-title">{tr(run.status === 'recovering' ? 'Routine recovering' : 'Routine failed')}</span>
                  <span className="inbox-row-sub">{run.detail || tr('Open the bot to see what happened')}</span>
                </span>
                <time>{when(run.startedAt)}</time>
              </button>
            ))}
          </section>
        ))}
      </div>

      {boxes.length > 0 ? (
        <div className="inbox-footer">
          <Button kind="secondary" onClick={() => onOpen(boxes[0]!.store.computer.id, boxes[0]!.bot.id)}>{tr('Open the first bot')}</Button>
        </div>
      ) : null}
    </div>
  )
}
