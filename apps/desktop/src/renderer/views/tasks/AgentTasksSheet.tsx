import { useCallback, useEffect, useState } from 'react'
import type { Bot } from '@shared/models'
import type { BotStore } from '../../store/bot-store'
import { Button, ChoicePicker, IconButton } from '../../components/Controls'
import { ModalHeader, SheetPage, usePresence } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import {
  actionsFor,
  countTasks,
  inFilter,
  statusFilterValue,
  statusLabel,
  statusOf,
  STATUS_FILTERS,
  type AgentTask,
  type StatusFilter,
} from './task-view-model'
import './tasks.css'

interface TaskMessage {
  id: string
  kind: string
  data?: { text?: string }
}

/**
 * The agent's parallel tasks for one bot. Everything here is host state: the host decides which
 * tasks run, routes results into the parent chat and remembers stops. This sheet only lists them
 * and sends the four intents (follow up, stop, archive, reopen) through the host API.
 */
export function AgentTasksSheet({ bot, store, onClose }: { bot: Bot; store: BotStore; onClose: () => void }) {
  const tr = useTr()
  const [tasks, setTasks] = useState<AgentTask[]>([])
  const [filter, setFilter] = useState<StatusFilter>('all')
  const [selected, setSelected] = useState<string | null>(null)
  const [messages, setMessages] = useState<TaskMessage[]>([])
  const [followUp, setFollowUp] = useState('')
  const [splitDraft, setSplitDraft] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const composer = usePresence(selected !== null)
  const client = useCallback(async () => store.ready(), [store])

  const load = useCallback(async () => {
    setBusy(true)
    try {
      const answer = await (await client()).call<{ tasks: AgentTask[] }>('taskList', {
        botId: bot.id,
        status: statusFilterValue(filter),
      })
      setTasks(answer.tasks)
      setError(null)
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason))
    }
    setBusy(false)
  }, [bot.id, client, filter])

  useEffect(() => {
    void load()
    const timer = setInterval(() => void load(), 4_000)
    return () => clearInterval(timer)
  }, [load])

  const act = async (method: string, body: Record<string, unknown>) => {
    setBusy(true)
    try {
      await (await client()).call(method, body)
      await load()
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason))
    }
    setBusy(false)
  }

  const open = async (task: AgentTask) => {
    setSelected(task.id)
    setFollowUp('')
    try {
      const answer = await (await client()).call<{ entries: TaskMessage[] }>('taskReadPage', { taskId: task.id })
      setMessages(answer.entries)
      setError(null)
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason))
    }
  }

  const split = async () => {
    const lines = splitDraft.split('\n').map((line) => line.trim()).filter(Boolean)
    if (!lines.length) return
    const titles = lines.map((line) => {
      const separator = line.indexOf(':')
      return separator > 0 ? line.slice(0, separator).trim() : `Task ${lines.indexOf(line) + 1}`
    })
    await act('taskSplit', {
      botId: bot.id,
      parentChat: bot.id,
      tasks: lines.map((line, index) => {
        const separator = line.indexOf(':')
        return { name: titles[index], text: separator > 0 ? line.slice(separator + 1).trim() : line }
      }),
    })
    setSplitDraft('')
  }

  const shown = tasks.filter((task) => inFilter(task, filter))
  const counts = countTasks(tasks)
  const selectedTask = tasks.find((task) => task.id === selected) ?? null

  return (
    <SheetPage className="agent-tasks-sheet">
      <ModalHeader title={tr('Agent tasks')} subtitle={bot.name} onClose={onClose} closeTitle={tr('Close')} />
      <VStack gap={14}>
        <HStack gap={8}>
          <Text weight="semibold" grow>{tr('Parallel work')}</Text>
          <Text size="caption" tone="secondary">
            {tr(`${counts.active} active · ${counts.queued} waiting`)}
          </Text>
          <ChoicePicker
            selection={filter}
            options={STATUS_FILTERS.map((option) => ({ id: option.id, label: tr(option.label) }))}
            onChange={setFilter}
          />
        </HStack>

        {shown.length ? (
          <VStack gap={6}>
            {shown.map((task) => {
              const actions = actionsFor(task)
              return (
                <VStack key={task.id} gap={6} className="agent-task-row">
                  <HStack gap={8}>
                    <button className="agent-task-open press" onClick={() => void open(task)} disabled={busy}>
                      <Text weight="semibold" grow truncate>{task.name}</Text>
                      <Text size="caption" tone={task.unread ? 'attention' : 'secondary'}>
                        {statusLabel(statusOf(task))}
                      </Text>
                    </button>
                    {actions.stop ? (
                      <IconButton title={tr('Stop')} icon="stop.circle" disabled={busy} onClick={() => void act('taskStop', { taskId: task.id })} />
                    ) : null}
                    {actions.archive ? (
                      <IconButton title={tr('Archive')} icon="archivebox" disabled={busy} onClick={() => void act('taskClose', { taskId: task.id, archived: true })} />
                    ) : null}
                    {actions.reopen ? (
                      <IconButton title={tr('Reopen')} icon="arrow.uturn.backward" disabled={busy} onClick={() => void act('taskReopen', { taskId: task.id })} />
                    ) : null}
                  </HStack>
                </VStack>
              )
            })}
          </VStack>
        ) : (
          <Text size="caption" tone="tertiary">{tr('No tasks yet. Split work into a few lines below.')}</Text>
        )}

        {composer.mounted ? (
          <VStack gap={8} className={`agent-task-detail ${composer.shown ? 'shown' : ''}`}>
            <Text weight="semibold">{selectedTask?.name ?? ''}</Text>
            {messages.length ? (
              <VStack gap={4}>
                {messages.map((message) => (
                  <Text key={message.id} size="caption" tone={message.kind === 'agent' ? 'text' : 'secondary'} lines={4}>
                    {message.data?.text ?? ''}
                  </Text>
                ))}
              </VStack>
            ) : <Text size="caption" tone="tertiary">{tr('No messages yet.')}</Text>}
            {selectedTask && actionsFor(selectedTask).followUp ? (
              <HStack gap={8}>
                <input
                  className="agent-task-input"
                  value={followUp}
                  placeholder={tr('Send a follow-up to this task')}
                  onChange={(event) => setFollowUp(event.target.value)}
                />
                <Button
                  kind="secondary"
                  disabled={busy || !followUp.trim()}
                  onClick={() => {
                    void act('taskFollowUp', { botId: bot.id, taskId: selectedTask.id, text: followUp }).then(() => setFollowUp(''))
                  }}
                >
                  {tr('Send')}
                </Button>
              </HStack>
            ) : null}
            <Button kind="secondary" onClick={() => setSelected(null)}>{tr('Close task')}</Button>
          </VStack>
        ) : null}

        <VStack gap={8}>
          <Text size="caption" tone="secondary">{tr('One task per line: Name: what it should do. Up to four run at once.')}</Text>
          <textarea
            className="agent-task-input"
            rows={3}
            value={splitDraft}
            placeholder={tr('Review: check the build\nDraft: write the release note')}
            onChange={(event) => setSplitDraft(event.target.value)}
          />
          <Button kind="secondary" disabled={busy || !splitDraft.trim()} onClick={() => void split()}>
            {tr('Split work')}
          </Button>
        </VStack>

        <HStack gap={8}>
          <Button kind="secondary" disabled={busy} onClick={() => void load()}>{tr('Refresh')}</Button>
          <Button kind="secondary" onClick={onClose}>{tr('Done')}</Button>
        </HStack>
        {error ? <Text size="caption" role="alert" tone="danger">{error}</Text> : null}
      </VStack>
    </SheetPage>
  )
}
