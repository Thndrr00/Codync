/** What the Tasks surface shows for a host task. Pure: no store, no host calls, no React. */

export interface AgentTask {
  id: string
  botId: string
  name: string
  state: 'open' | 'closed' | 'archived'
  root: string
  startedAt?: number
  closedAt?: number
  unread?: number
  rev?: number
  updatedAt?: number
  parentChat?: string
  parentThread?: string
  run?: 'idle' | 'queued' | 'running' | 'stopped'
  routedSeq?: number
}

export type TaskStatus = 'queued' | 'running' | 'waiting' | 'stopped' | 'paused' | 'archived'

export interface TaskActions {
  open: boolean
  followUp: boolean
  stop: boolean
  archive: boolean
  reopen: boolean
}

const STATUS_LABEL: Record<TaskStatus, string> = {
  queued: 'Waiting for a slot',
  running: 'Running',
  waiting: 'Waiting for a reply',
  stopped: 'Stopped',
  paused: 'Paused',
  archived: 'Archived',
}

/** State and schedule agree: an archived task never reports as running, even after a stale event. */
export function statusOf(task: AgentTask): TaskStatus {
  if (task.state === 'archived') return 'archived'
  if (task.run === 'stopped') return 'stopped'
  if (task.state === 'closed') return 'paused'
  if (task.run === 'queued') return 'queued'
  if (task.run === 'running') return 'running'
  return 'waiting'
}

export function statusLabel(status: TaskStatus): string {
  return STATUS_LABEL[status]
}

/** OpenBot 010: branch only from a finished task, never while its turn is queued or running. */
export function canBranch(task: AgentTask): boolean {
  const status = statusOf(task)
  return task.state !== 'archived' && status !== 'queued' && status !== 'running'
}

/** What the rows offer. A stopped task can be followed up but never restarted on its own. */
export function actionsFor(task: AgentTask): TaskActions {
  const status = statusOf(task)
  return {
    open: status !== 'archived',
    followUp: status !== 'archived' && status !== 'paused',
    stop: status === 'queued' || status === 'running' || status === 'waiting',
    archive: status !== 'archived',
    reopen: status === 'paused' || status === 'archived',
  }
}

export interface TaskCounts {
  active: number
  queued: number
  unread: number
}

export function countTasks(tasks: AgentTask[]): TaskCounts {
  return tasks.reduce<TaskCounts>(
    (counts, task) => {
      const status = statusOf(task)
      if (status === 'running' || status === 'waiting') counts.active += 1
      if (status === 'queued') counts.queued += 1
      counts.unread += task.unread ?? 0
      return counts
    },
    { active: 0, queued: 0, unread: 0 },
  )
}

/** The status filter options, kept in one place so the picker and the tests agree. */
export const STATUS_FILTERS = [
  { id: 'all', label: 'All' },
  { id: 'open', label: 'Open' },
  { id: 'closed', label: 'Paused' },
  { id: 'archived', label: 'Archived' },
] as const

export type StatusFilter = (typeof STATUS_FILTERS)[number]['id']

/** OpenBot 010: one id per branch click, so a double-click replays the first branch on the host. */
export function branchRequestId(): string {
  return `branch-${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`
}

export function statusFilterValue(filter: StatusFilter): AgentTask['state'] | undefined {
  return filter === 'all' ? undefined : filter
}

export function inFilter(task: AgentTask, filter: StatusFilter): boolean {
  return filter === 'all' || task.state === filter
}
