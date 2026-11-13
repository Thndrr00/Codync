import { useEffect, useState } from 'react'
import { Button, Spinner } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { ModalHeader, Sheet } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { errorText } from '../bots/drafts'
import type { BotStore } from '../../store/bot-store'
import './notifications.css'

interface Notice {
  id: string
  botId: string
  botName: string | null
  text: string
  style?: string
  health?: boolean
  at: number
}

interface Row extends Notice {
  computerId: string
  computerName: string
}

/**
 * The alerts and notices the host wrote across its chats, newest first: memory and disk warnings,
 * budget gates, sign-ins and other things worth knowing later. It reads what the chats already
 * show; nothing new is stored for it.
 */
export function NotificationCenter({ stores, open, onClose, onOpen }: {
  stores: BotStore[]
  open: boolean
  onClose: () => void
  onOpen: (computerId: string, botId: string) => void
}) {
  const [rows, setRows] = useState<Row[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [rev, setRev] = useState(0)

  useEffect(() => {
    if (!open) return
    let stopped = false
    const load = async () => {
      setLoading(true)
      try {
        const found = await Promise.all(
          stores.map(async (store) => {
            if (store.connection.kind !== 'online' || store.mismatch) return []
            const result = await (await store.ready()).call<{ notices: Notice[] }>('recentNotices', { days: 14, limit: 80 })
            return result.notices.map((notice): Row => ({
              ...notice,
              computerId: store.computer.id,
              computerName: store.computer.name,
            }))
          }),
        )
        if (stopped) return
        setRows(found.flat().sort((a, b) => b.at - a.at).slice(0, 200))
        setError(null)
      } catch (e) {
        if (!stopped) setError(errorText(e))
      } finally {
        if (!stopped) setLoading(false)
      }
    }
    void load()
    return () => {
      stopped = true
    }
  }, [open, stores, rev])

  return (
    <Sheet open={open} onClose={onClose} width={540} maxHeight="80vh">
      <VStack gap={12}>
        <ModalHeader
          icon="bell.badge"
          title="Notifications"
          subtitle="Notices from your bots' chats over the last two weeks."
          onClose={onClose}
        />
        {error ? <Text size="caption" tone="danger">{error}</Text> : null}
        {loading && rows.length === 0 ? (
          <HStack gap={8} align="center"><Spinner size={12} /><Text size="caption" tone="secondary">Loading…</Text></HStack>
        ) : null}
        {!loading && rows.length === 0 ? (
          <Text size="callout" tone="secondary">Nothing has been announced recently. Health, budget and sign-in notices land here.</Text>
        ) : null}
        <div className="notifications-scroll">
          {rows.map((row) => (
            <button
              key={`${row.computerId}:${row.id}`}
              type="button"
              className="notification-row press"
              onClick={() => {
                onClose()
                onOpen(row.computerId, row.botId)
              }}
            >
              <span className={`notification-icon ${row.style === 'error' ? 'error' : row.health ? 'attention' : ''}`}>
                <Icon name={row.style === 'error' ? 'exclamationmark.triangle' : row.health ? 'gauge.with.dots.needle.33percent' : 'info.circle'} size={13} />
              </span>
              <VStack gap={2} grow>
                <Text size="caption" tone="secondary">
                  {row.botName ?? row.botId}
                  {stores.length > 1 ? ` · ${row.computerName}` : ''}
                  {' · '}
                  {new Date(row.at).toLocaleString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })}
                </Text>
                <Text size="callout" className="notification-text">{row.text}</Text>
              </VStack>
              <Icon name="chevron.right" size={11} />
            </button>
          ))}
        </div>
        <HStack justify="end">
          <Button kind="secondary" disabled={loading} onClick={() => setRev((n) => n + 1)}>Refresh</Button>
        </HStack>
      </VStack>
    </Sheet>
  )
}

