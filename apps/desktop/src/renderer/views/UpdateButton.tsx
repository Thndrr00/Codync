import { useEffect, useState } from 'react'
import type { UpdateState } from '@shared/ipc'
import { updateAction } from '@shared/update-action'
import { Button, ProgressRing, Spinner } from '../components/Controls'
import { Icon } from '../components/Icon'
import { VStack } from '../components/Stack'
import { Text } from '../components/Text'
import { useStore } from '../store/context'
import { liveClient } from './settings/parts'

/** The app's update state, kept live. */
export function useUpdates() {
  const [updates, setUpdates] = useState<UpdateState | null>(null)
  useEffect(() => {
    void window.codync.updates.state().then(setUpdates)
    return window.codync.updates.onChange(setUpdates)
  }, [])
  return updates
}

/** What this computer's host is waiting for before a requested restart, kept live. */
export function useHostRestartWait() {
  const store = useStore()
  const client = liveClient(store)
  const [waitingFor, setWaitingFor] = useState<string[]>([])
  useEffect(() => {
    if (!client || !store.isLoopback) return
    let active = true
    const refresh = () => {
      void client.call<{ waitingFor?: string[] }>('hostUpdateStatus')
        .then((status) => { if (active) setWaitingFor(status.waitingFor ?? []) })
        .catch(() => { if (active) setWaitingFor([]) })
    }
    refresh()
    const timer = setInterval(refresh, 10_000)
    return () => { active = false; clearInterval(timer) }
  }, [client, store.isLoopback])
  const cancel = () => {
    if (!client) return
    void client.call('cancelHostRestartWait').then(() => setWaitingFor([])).catch(() => {})
  }
  return { waitingFor, cancel }
}

/** Check, update, download progress, then Restart to update: whatever the update state offers. */
export function UpdateButton({ updates, kind = 'secondary' }: { updates: UpdateState; kind?: 'primary' | 'secondary' }) {
  const action = updateAction(updates)
  const { waitingFor, cancel } = useHostRestartWait()
  const waiting = waitingFor.length > 0
  return (
    <VStack gap={6} align="flex-start">
      <Button
        kind={updates.staged ? 'primary' : kind}
        onClick={() => window.codync.updates.check()}
        disabled={!action.enabled}
        title={waiting ? 'This computer is finishing work before it restarts' : undefined}
      >
        {action.label}
      </Button>
      {waiting ? (
        <>
          <Text size="caption" tone="secondary">{`Waiting to restart: ${waitingFor.join(', ')}`}</Text>
          <Button kind="secondary" onClick={cancel}>Cancel restart wait</Button>
        </>
      ) : null}
    </VStack>
  )
}

/**
 * The sidebar's Update: one tap downloads the release while Codync stays usable (a ring fills as
 * it downloads), then restarts into it.
 */
export function UpdateBadge({ size }: { size: number }) {
  const updates = useUpdates()
  if (!updates?.supported || !updates.availableVersion) return null
  const action = updateAction(updates)
  return (
    <button
      className="update-badge"
      style={{ width: size, height: size }}
      title={action.label}
      aria-label={action.label}
      aria-busy={updates.installing || updates.downloadPercent !== null}
      disabled={updates.installing}
      onClick={() => window.codync.updates.install()}
    >
      <BadgeIcon updates={updates} size={size} />
    </button>
  )
}

function BadgeIcon({ updates, size }: { updates: UpdateState; size: number }) {
  if (updates.installing) return <Spinner size={size * 0.5} />
  if (updates.downloadPercent !== null) return <ProgressRing value={updates.downloadPercent / 100} size={size * 0.5} />
  return <Icon name="arrow.down" size={size * 0.42} weight="semibold" />
}
