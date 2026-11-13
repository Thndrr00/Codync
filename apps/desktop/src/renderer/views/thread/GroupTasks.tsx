import { useRef, useState } from 'react'
import { isGroup, taskStopped, type Bot, type GroupTask } from '@shared/models'
import { AnchoredMenu, type MenuItem } from '../../components/Menu'
import { Icon } from '../../components/Icon'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'
import './group-tasks.css'

/**
 * A channel's request tree over the composer, as iOS `GroupTasksCard` and the Android `GroupTasksCard`: the request,
 * the members' assignments and their children, each with its state. Stop pauses a row and its branch; a stopped row
 * offers Continue, Reassign (another member) and Cancel.
 */
export function GroupTasks({ bot }: { bot: Bot }) {
  const store = useStore()
  const tr = useTr()
  const [assigning, setAssigning] = useState<string | null>(null)
  const anchor = useRef<HTMLButtonElement>(null)
  const tasks = (bot.tasks ?? []).filter((task) => task.state !== 'cancelled')
  if (!isGroup(bot) || tasks.length === 0) return null
  const picked = tasks.find((task) => task.id === assigning) ?? null

  const act = async (task: GroupTask, action: string, ownerId?: string) => {
    const error = await store.groupTask(bot.id, task.id, action, ownerId)
    if (error) store.setError(error)
    setAssigning(null)
  }
  const items = (): MenuItem[] =>
    bot.members
      .map((id) => store.bots.get(id))
      .filter((member): member is Bot => !!member && member.id !== picked?.ownerId)
      .map((member) => ({ title: member.name, verbatim: true, action: () => picked && void act(picked, 'reassign', member.id) }))

  return (
    <div className="group-tasks">
      <VStack gap={4} className="group-tasks-card">
        <HStack gap={6}>
          <Icon name="person.2" size={11} weight="semibold" color="var(--secondary)" />
          <Text size="caption2" tone="tertiary">{tr('Waiting for sub-tasks')}</Text>
        </HStack>
        {tasks.map((task) => (
          <VStack key={task.id} gap={2} className={`group-task ${task.state}`}>
            <HStack gap={4}>
              <Text size="caption" weight="semibold" tone="text">{task.ownerName ?? tr('The lead')}</Text>
              <Text size="caption2" tone="tertiary">{stateLabel(task.state, tr)}</Text>
              {task.instruction ? <Text size="caption2" tone="tertiary" className="group-task-line" lines={1}>{task.instruction}</Text> : null}
            </HStack>
            <HStack gap={10}>
              {taskStopped(task) ? (
                <>
                  <button className="press group-task-action" onClick={() => void act(task, 'continue')}>
                    <Icon name="play.fill" size={9} weight="semibold" color="var(--secondary)" />
                    {tr('Continue')}
                  </button>
                  <button
                    ref={task.id === assigning ? anchor : undefined}
                    className="press group-task-action"
                    onClick={() => setAssigning(task.id === assigning ? null : task.id)}
                  >
                    <Icon name="arrow.left.arrow.right" size={9} weight="semibold" color="var(--secondary)" />
                    {tr('Reassign')}
                  </button>
                  <button className="press group-task-action" onClick={() => void act(task, 'cancel')}>
                    <Icon name="xmark" size={9} weight="semibold" color="var(--secondary)" />
                    {tr('Cancel')}
                  </button>
                </>
              ) : task.state === 'queued' || task.state === 'running' ? (
                <button className="press group-task-action" onClick={() => void act(task, 'stop')}>
                  <Icon name="stop.fill" size={9} weight="semibold" color="var(--secondary)" />
                  {tr('Stop')}
                </button>
              ) : null}
            </HStack>
          </VStack>
        ))}
      </VStack>
      <AnchoredMenu open={!!picked} onClose={() => setAssigning(null)} anchor={anchor} items={items} />
    </div>
  )
}

function stateLabel(state: string, tr: ReturnType<typeof useTr>): string {
  switch (state) {
    case 'queued':
      return tr('waiting')
    case 'running':
      return tr('working')
    case 'completed':
      return tr('done')
    case 'failed':
      return tr('failed')
    case 'paused':
      return tr('stopped')
    default:
      return tr('cancelled')
  }
}
