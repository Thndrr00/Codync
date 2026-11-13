import { useEffect, useState } from 'react'
import type { IslandSummary } from '@shared/island'
import { Button } from '../components/Controls'
import { Icon } from '../components/Icon'
import { HStack, VStack } from '../components/Stack'
import { Text } from '../components/Text'
import './dock.css'

const statusLabel: Record<string, string> = {
  idle: 'Idle',
  working: 'Working',
  needsInput: 'Needs you',
  error: 'Error',
  offline: 'Offline',
}

/**
 * The agent dock: a small always-on-top window with the same bots and approvals the notch island
 * gets from the chat window. It does not connect to a host itself; the chat owns the stores and sends
 * the summary.
 */
export function DockWindow() {
  const [summary, setSummary] = useState<IslandSummary | null>(null)
  useEffect(() => {
    let alive = true
    void window.codync.island.state().then((state) => {
      if (alive) setSummary(state.summary)
    })
    const off = window.codync.island.onSummary((next) => setSummary(next))
    return () => {
      alive = false
      off()
    }
  }, [])
  const bots = summary?.bots ?? []
  const approvals = summary?.permissions ?? []
  return (
    <div className="dock">
      <HStack gap={8} align="center" className="dock-header">
        <Icon name="square.grid.2x2" size={14} />
        <Text weight="semibold" grow>Agent dock</Text>
        <Text size="caption" tone="tertiary">{bots.length}</Text>
      </HStack>
      {summary?.health ? <Text size="caption" tone="attention" className="dock-health">{summary.health}</Text> : null}
      {approvals.map((permission) => (
        <VStack key={permission.entryId} gap={4} className="dock-approval">
          <Text size="caption" tone="secondary">{permission.bot.name} · Approval</Text>
          <Text size="callout">{permission.title}</Text>
          <HStack gap={6} wrap>
            {permission.options.map((option) => (
              <Button
                key={option.id}
                kind="secondary"
                disabled={permission.answering !== null}
                onClick={() => window.codync.island.respond(permission.ref, permission.entryId, option.id)}
              >
                {option.label}
              </Button>
            ))}
          </HStack>
        </VStack>
      ))}
      <div className="dock-list">
        {bots.length === 0 ? (
          <Text size="caption" tone="secondary">No bots yet. Open Codync to create one.</Text>
        ) : (
          bots.map((bot) => (
            <button key={bot.ref} type="button" className="dock-row press" onClick={() => window.codync.island.openBot(bot.ref)}>
              <span className={`dock-dot ${bot.status}`} />
              <VStack gap={1} grow>
                <Text size="callout" weight="medium" truncate>{bot.name}</Text>
                <Text size="caption" tone="secondary" truncate>
                  {statusLabel[bot.status] ?? bot.status}{bot.detail ? ` · ${bot.detail}` : ''}
                </Text>
              </VStack>
              <Icon name="chevron.right" size={10} />
            </button>
          ))
        )}
      </div>
    </div>
  )
}
