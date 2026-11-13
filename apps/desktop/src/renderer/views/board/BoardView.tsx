import { useEffect, useMemo, useState } from 'react'
import { isGroup } from '@shared/models'
import { BotAvatar } from '../../components/Avatar'
import { Button, ChoicePicker, DropdownMenu, Spinner } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { ModalHeader, Sheet } from '../../components/Overlay'
import { HStack, Spacer, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import type { BotStore } from '../../store/bot-store'
import { errorText } from '../bots/drafts'
import { Field } from '../marketplace/pieces'
import './board.css'

interface BoardTask {
  id: string
  title: string
  detail: string
  status: string
  botId?: string
  routineId?: string
  dueAt?: number
  createdAt: number
  updatedAt: number
}

const COLUMNS: { id: string; label: string; hint: string }[] = [
  { id: 'todo', label: 'To do', hint: 'Not started' },
  { id: 'doing', label: 'Doing', hint: 'In progress' },
  { id: 'waiting', label: 'Waiting', hint: 'On someone else' },
  { id: 'done', label: 'Done', hint: 'Finished' },
]

const dueText = (ms?: number) => {
  if (!ms) return null
  const late = ms < Date.now()
  return { text: new Date(ms).toLocaleDateString(undefined, { month: 'short', day: 'numeric' }), late }
}

/**
 * The computer's board: one column per state, each card naming the bot that owns it. The host keeps
 * the cards, so every window sees the same board. Moving a card and adding one are the person's
 * actions; a card opens its bot's chat so the work itself happens there.
 */
export function BoardView({ stores, onOpen }: { stores: BotStore[]; onOpen: (computerId: string, botId: string) => void }) {
  const tr = useTr()
  const online = stores.filter((store) => store.connection.kind === 'online' && !store.mismatch)
  const [computer, setComputer] = useState(() => online[0]?.computer.id ?? '')
  const store = online.find((s) => s.computer.id === computer) ?? online[0]
  const [tasks, setTasks] = useState<BoardTask[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [title, setTitle] = useState('')
  const [owner, setOwner] = useState('')
  const [routine, setRoutine] = useState('')
  const [routines, setRoutines] = useState<{ id: string; name: string }[]>([])
  const [query, setQuery] = useState('')
  const [ownerFilter, setOwnerFilter] = useState('')
  const [busy, setBusy] = useState(false)
  const [editing, setEditing] = useState<BoardTask | null>(null)
  const [editTitle, setEditTitle] = useState('')
  const [editDetail, setEditDetail] = useState('')
  const [editDue, setEditDue] = useState('')

  useEffect(() => {
    if (!store) return
    let stopped = false
    const load = async () => {
      try {
        const result = await (await store.ready()).call<{ tasks: BoardTask[] }>('boardTasks', {})
        if (stopped) return
        setTasks(result.tasks)
        setError(null)
      } catch (e) {
        if (!stopped && !store.isOffline) setError(errorText(e))
      } finally {
        if (!stopped) setLoading(false)
      }
    }
    void load()
    const timer = setInterval(load, 10_000)
    return () => {
      stopped = true
      clearInterval(timer)
    }
  }, [store])

  const bots = useMemo(() => (store ? [...store.bots.values()].filter((bot) => !isGroup(bot) && !bot.hidden) : []), [store])
  useEffect(() => {
    setRoutine('')
    if (!store || !owner) {
      setRoutines([])
      return
    }
    let stopped = false
    void (async () => {
      try {
        const result = await (await store.ready()).call<{ routines: { id: string; name: string }[] }>('routines', { botId: owner })
        if (!stopped) setRoutines(result.routines)
      } catch {
        if (!stopped) setRoutines([])
      }
    })()
    return () => {
      stopped = true
    }
  }, [store, owner])
  const q = query.trim().toLowerCase()
  const visible = tasks.filter((task) => {
    if (ownerFilter && task.botId !== ownerFilter) return false
    return !q || `${task.title}\n${task.detail}`.toLowerCase().includes(q)
  })
  const columns = COLUMNS.map((column) => ({ ...column, cards: visible.filter((task) => task.status === column.id) }))

  const reload = async () => {
    if (!store) return
    const result = await (await store.ready()).call<{ tasks: BoardTask[] }>('boardTasks', {})
    setTasks(result.tasks)
  }

  const add = async () => {
    if (!store || !title.trim()) return
    setBusy(true)
    setError(null)
    try {
      await (await store.ready()).call('saveBoardTask', { title: title.trim(), detail: '', status: 'todo', botId: owner || null, routineId: routine || null })
      setTitle('')
      await reload()
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const move = async (task: BoardTask, status: string) => {
    if (!store) return
    setTasks((list) => list.map((item) => (item.id === task.id ? { ...item, status } : item)))
    try {
      await (await store.ready()).call('moveBoardTask', { id: task.id, status })
    } catch (e) {
      setError(errorText(e))
      await reload().catch(() => {})
    }
  }

  const remove = async (task: BoardTask) => {
    if (!store) return
    setTasks((list) => list.filter((item) => item.id !== task.id))
    try {
      await (await store.ready()).call('deleteBoardTask', { id: task.id })
    } catch (e) {
      setError(errorText(e))
      await reload().catch(() => {})
    }
  }

  const openEdit = (task: BoardTask) => {
    setEditing(task)
    setEditTitle(task.title)
    setEditDetail(task.detail)
    const due = task.dueAt ? new Date(task.dueAt) : null
    const pad = (value: number) => String(value).padStart(2, '0')
    setEditDue(due ? `${due.getFullYear()}-${pad(due.getMonth() + 1)}-${pad(due.getDate())}` : '')
  }

  const saveEdit = async () => {
    if (!store || !editing || !editTitle.trim()) return
    let dueAt: number | null = null
    if (editDue.trim()) {
      const at = new Date(`${editDue.trim()}T12:00`).getTime()
      if (!Number.isFinite(at)) {
        setError('The due date is a day, like 2026-11-20.')
        return
      }
      dueAt = at
    }
    setBusy(true)
    setError(null)
    try {
      await (await store.ready()).call('saveBoardTask', {
        id: editing.id,
        title: editTitle.trim(),
        detail: editDetail.trim(),
        status: editing.status,
        botId: editing.botId ?? null,
        routineId: editing.routineId ?? null,
        dueAt,
      })
      setEditing(null)
      await reload()
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="board-page">
      <header className="board-header">
        <VStack gap={2}>
          <Text size="title3" weight="semibold">{tr('Board')}</Text>
          <Text size="caption" tone="secondary">
            {tasks.length === 0 ? tr('No cards yet. Add the first one.') : `${tasks.length} ${tr(tasks.length === 1 ? 'card' : 'cards')}`}
          </Text>
        </VStack>
        <Spacer />
        {loading ? <Spinner /> : null}
        <div className="board-search">
          <Field label="Search cards" value={query} onChange={setQuery} placeholder={tr('Search cards')} />
        </div>
        {bots.length > 0 ? (
          <ChoicePicker
            selection={ownerFilter}
            options={[{ id: '', label: tr('Every bot') }, ...bots.map((bot) => ({ id: bot.id, label: bot.name }))]}
            onChange={setOwnerFilter}
          />
        ) : null}
        {online.length > 1 ? (
          <ChoicePicker
            selection={store?.computer.id ?? ''}
            options={online.map((s) => ({ id: s.computer.id, label: s.computer.name }))}
            onChange={setComputer}
          />
        ) : null}
      </header>

      {error ? <Text size="caption" tone="danger">{error}</Text> : null}

      {!store ? (
        <Text size="callout" tone="secondary">{tr('Start this computer’s host to use the board.')}</Text>
      ) : (
        <>
          <HStack gap={8} className="board-add" align="end">
            <VStack gap={4} grow>
              <Text size="caption" tone="secondary">{tr('New card')}</Text>
              <Field label="New card" value={title} onChange={setTitle} placeholder="What needs doing?" autoFocus />
            </VStack>
            <ChoicePicker
              selection={owner}
              options={[{ id: '', label: tr('No bot') }, ...bots.map((bot) => ({ id: bot.id, label: bot.name }))]}
              onChange={setOwner}
            />
            {routines.length > 0 ? (
              <ChoicePicker
                selection={routine}
                options={[{ id: '', label: tr('No routine') }, ...routines.map((item) => ({ id: item.id, label: item.name }))]}
                onChange={setRoutine}
              />
            ) : null}
            <Button disabled={busy || !title.trim()} onClick={() => void add()}>{tr('Add')}</Button>
          </HStack>

          <div className="board-columns">
            {columns.map((column) => (
              <section
                key={column.id}
                className={`board-column ${column.id}`}
                aria-label={tr(column.label)}
                onDragOver={(event) => {
                  if (event.dataTransfer.types.includes('text/board-card')) event.preventDefault()
                }}
                onDrop={(event) => {
                  const id = event.dataTransfer.getData('text/board-card')
                  const task = tasks.find((item) => item.id === id)
                  if (task && task.status !== column.id) {
                    event.preventDefault()
                    void move(task, column.id)
                  }
                }}
              >
                <HStack gap={6} align="center" className="board-column-head">
                  <Text size="callout" weight="semibold" grow>{tr(column.label)}</Text>
                  <Text size="caption" tone="tertiary">{column.cards.length}</Text>
                </HStack>
                <Text size="caption" tone="tertiary" className="board-column-hint">{tr(column.hint)}</Text>
                {column.cards.map((task) => {
                  const due = dueText(task.dueAt)
                  const bot = task.botId ? store.bots.get(task.botId) : null
                  return (
                    <article
                      key={task.id}
                      className="board-card"
                      draggable
                      onDragStart={(event) => {
                        event.dataTransfer.setData('text/board-card', task.id)
                        event.dataTransfer.effectAllowed = 'move'
                      }}
                    >
                      <HStack gap={6} align="start">
                        <Text size="callout" grow className="board-card-title">{task.title}</Text>
                        <DropdownMenu className="board-card-menu" title={tr('Card actions')} items={() => [
                          { title: tr('Edit card…'), icon: 'pencil', action: () => openEdit(task) },
                          ...COLUMNS.filter((c) => c.id !== task.status).map((c) => ({ title: tr(`Move to ${c.label}`), icon: 'arrow.right', action: () => void move(task, c.id) })),
                          { title: tr('Delete'), icon: 'trash', action: () => void remove(task) },
                        ]}>
                          <Icon name="ellipsis" size={13} color="var(--secondary)" />
                        </DropdownMenu>
                      </HStack>
                      {task.detail ? <Text size="caption" tone="secondary" className="board-card-detail">{task.detail}</Text> : null}
                      <HStack gap={6} align="center">
                        {bot ? (
                          <button type="button" className="board-owner press" onClick={() => onOpen(store.computer.id, bot.id)}>
                            <BotAvatar bot={bot} size={16} animated={false} />
                            <span>{bot.name}</span>
                          </button>
                        ) : null}
                        {task.routineId ? <Text size="caption" tone="tertiary">↻ {tr('Routine')}</Text> : null}
                        {due ? <Text size="caption" tone={due.late ? 'danger' : 'tertiary'}>{due.text}</Text> : null}
                      </HStack>
                    </article>
                  )
                })}
                {column.cards.length === 0 ? <Text size="caption" tone="tertiary">{tr('Nothing here.')}</Text> : null}
              </section>
            ))}
          </div>
        </>
      )}
      <Sheet open={editing !== null} onClose={() => setEditing(null)} width={480}>
        <VStack gap={12}>
          <ModalHeader icon="checklist" title="Edit card" onClose={() => setEditing(null)} />
          <VStack gap={4}>
            <Text size="caption" tone="secondary">{tr('Title')}</Text>
            <Field label="Title" value={editTitle} onChange={setEditTitle} placeholder={tr('What needs doing?')} />
          </VStack>
          <VStack gap={4}>
            <Text size="caption" tone="secondary">{tr('Detail')}</Text>
            <Field label="Detail" value={editDetail} onChange={setEditDetail} lines={[3, 8]} placeholder={tr('Anything worth keeping with the card')} />
          </VStack>
          <VStack gap={4}>
            <Text size="caption" tone="secondary">{tr('Due date (year-month-day, optional)')}</Text>
            <Field label="Due date" value={editDue} onChange={setEditDue} placeholder="2026-11-20" />
          </VStack>
          <HStack justify="end" gap={8}>
            <Button kind="secondary" onClick={() => setEditing(null)}>{tr('Cancel')}</Button>
            <Button disabled={busy || !editTitle.trim()} onClick={() => void saveEdit()}>{tr('Save')}</Button>
          </HStack>
        </VStack>
      </Sheet>
    </div>
  )
}
