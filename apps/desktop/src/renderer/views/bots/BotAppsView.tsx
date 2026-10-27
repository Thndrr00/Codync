import { useCallback, useEffect, useState } from 'react'
import {
  collectDeliverables,
  collectPendingDecisions,
  type BotApp,
  type MessageBookmark,
  type OpenItem,
} from '@shared/models'
import { BotAppPanel } from '../../components/BotAppPanel'
import { Button, CardSection, IconButton, Spinner } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { SpinningCounter } from '../../components/SpinningCounter'
import { HStack, Spacer, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { ToastContainer, useToasts } from '../../components/ToastContainer'
import { useCardTilt } from '../../lib/use-card-tilt'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'

function MiniAppRow({
  app,
  onOpen,
  onDelete,
  tr,
}: {
  app: BotApp
  onOpen: () => void
  onDelete: () => void
  tr: (s: string) => string
}) {
  const tilt = useCardTilt(4)
  return (
    <div
      ref={tilt.ref}
      onPointerMove={tilt.onPointerMove}
      onPointerLeave={tilt.onPointerLeave}
      className="t-tilt"
    >
      <div className="t-tilt-card" style={{ position: 'relative' }}>
        <div className="t-tilt-glare" />
        <HStack align="center" gap={10}>
          <Icon name="macwindow" size={15} color="var(--accent)" />
          <VStack gap={2} grow>
            <HStack align="center" gap={6}>
              <Text weight="medium" truncate>
                {app.title}
              </Text>
              <span className="t-tab-badge">v{app.version}</span>
            </HStack>
            {app.description ? (
              <Text size="caption" tone="secondary" truncate>
                {app.description}
              </Text>
            ) : null}
          </VStack>
          <Button kind="secondary" onClick={onOpen}>
            {tr('Open')}
          </Button>
          <IconButton title={tr('Delete app')} icon="trash" onClick={onDelete} />
        </HStack>
      </div>
    </div>
  )
}

/** Inspector card for a bot's sandboxed mini-apps, open items, pending decisions (N12), deliverables (N09), and bookmarks (N14). */
export function BotAppsCard({ botId }: { botId: string }) {
  const tr = useTr()
  const store = useStore()
  const toasts = useToasts()
  const [apps, setApps] = useState<BotApp[]>([])
  const [openItems, setOpenItems] = useState<OpenItem[]>([])
  const [bookmarks, setBookmarks] = useState<MessageBookmark[]>([])
  const [editingNoteId, setEditingNoteId] = useState<string | null>(null)
  const [noteDraft, setNoteDraft] = useState('')
  const [loading, setLoading] = useState(true)
  const [activeApp, setActiveApp] = useState<BotApp | null>(null)

  const entries = store.allEntries(botId)
  const pendingDecisions = collectPendingDecisions(entries)
  const deliverables = collectDeliverables(entries)

  const refresh = useCallback(async () => {
    setLoading(true)
    try {
      const [loadedApps, loadedItems, loadedBookmarks] = await Promise.all([
        store.listBotApps(botId).catch(() => []),
        store.listOpenItems(botId).catch(() => []),
        store.listBookmarks(botId).catch(() => []),
      ])
      setApps(loadedApps)
      setOpenItems(loadedItems)
      setBookmarks(loadedBookmarks)
      setActiveApp((prev) => (prev ? (loadedApps.find((a) => a.slug === prev.slug) ?? prev) : null))
    } finally {
      setLoading(false)
    }
  }, [store, botId])

  useEffect(() => {
    void refresh()
  }, [refresh])

  return (
    <div className="bot-form">
      <div className="bot-form-content">
        <CardSection
          title={tr('Mini-Apps')}
          footer={tr('Sandboxed HTML apps created by this bot with live WebSocket updates and capability links.')}
        >
          {loading && apps.length === 0 ? (
            <HStack justify="center" style={{ padding: 12 }}>
              <Spinner size={16} />
            </HStack>
          ) : apps.length === 0 ? (
            <Text tone="secondary">{tr('No mini-apps yet. Ask the bot to build an interactive dashboard, tracker, or tool.')}</Text>
          ) : (
            apps.map((app) => (
              <MiniAppRow
                key={app.slug}
                app={app}
                tr={tr}
                onOpen={() => setActiveApp(app)}
                onDelete={async () => {
                  await store.deleteBotApp(botId, app.slug)
                  toasts.push(tr('Mini-app deleted'), 'info')
                  await refresh()
                }}
              />
            ))
          )}
        </CardSection>

        <CardSection
          title={tr('Open Items')}
          footer={tr('Commitments, follow-ups, and tracked items monitored during proactive heartbeats.')}
        >
          <HStack align="center" gap={8}>
            <Text size="caption" tone="secondary">
              {tr('Active tracked items')}
            </Text>
            <Spacer />
            <SpinningCounter value={openItems.length} className="t-tab-badge" />
          </HStack>
          {openItems.length === 0 ? (
            <Text tone="secondary">{tr('No open items tracked right now.')}</Text>
          ) : (
            openItems.map((item) => (
              <HStack key={item.id} align="center" gap={10}>
                <Icon name="checklist" size={14} color="var(--secondary)" />
                <VStack gap={2} grow>
                  <Text weight="medium" truncate>
                    {item.title}
                  </Text>
                  <Text size="caption" tone="secondary" truncate>
                    {item.kind}
                    {item.detail ? ` · ${item.detail}` : ''}
                    {item.dueAt ? ` · Due ${new Date(item.dueAt).toLocaleString()}` : ''}
                  </Text>
                </VStack>
                <IconButton
                  title={tr('Mark resolved')}
                  icon="checkmark.circle"
                  onClick={async () => {
                    await store.dismissOpenItem(botId, item.id)
                    await refresh()
                  }}
                />
              </HStack>
            ))
          )}
        </CardSection>

        <CardSection
          title={tr('Pending Decisions')}
          footer={tr('Permissions, takeovers, and errors waiting on you (N12).')}
        >
          <HStack align="center" gap={8}>
            <Text size="caption" tone="secondary">
              {tr('Awaiting action')}
            </Text>
            <Spacer />
            <SpinningCounter value={pendingDecisions.length} className="t-tab-badge" />
          </HStack>
          {pendingDecisions.length === 0 ? (
            <Text tone="secondary">{tr('No pending approvals or incidents.')}</Text>
          ) : (
            pendingDecisions.map((dec) => {
              const entry = entries.find((e) => e.id === dec.entryId)
              return (
                <HStack key={dec.entryId} align="center" gap={10}>
                  <Icon
                    name={dec.kind === 'permission' || dec.kind === 'handoff' ? 'hand.raised.fill' : 'exclamationmark.triangle.fill'}
                    size={14}
                    color="var(--warning, #ff9f0a)"
                  />
                  <VStack gap={2} grow>
                    <Text weight="medium" truncate>
                      {dec.title}
                    </Text>
                    {dec.detail ? (
                      <Text size="caption" tone="secondary" truncate>
                        {dec.detail}
                      </Text>
                    ) : null}
                  </VStack>
                  {dec.kind === 'permission' && entry ? (
                    <>
                      {dec.denyOptionId ? (
                        <Button onClick={() => store.respond(entry, dec.denyOptionId ?? null)}>{tr('Deny')}</Button>
                      ) : null}
                      {dec.allowOptionId ? (
                        <Button kind="primary" onClick={() => store.respond(entry, dec.allowOptionId ?? null)}>
                          {tr('Approve')}
                        </Button>
                      ) : null}
                    </>
                  ) : null}
                </HStack>
              )
            })
          )}
        </CardSection>

        <CardSection
          title={tr('Deliverables Library')}
          footer={tr('Files and artifacts generated or shared in this conversation (N09).')}
        >
          {deliverables.length === 0 ? (
            <Text tone="secondary">{tr('No files shared in this conversation yet.')}</Text>
          ) : (
            deliverables.map((file) => (
              <HStack key={file.fileId} align="center" gap={10}>
                <Icon name="doc.fill" size={14} color="var(--accent)" />
                <VStack gap={2} grow>
                  <Text weight="medium" truncate>
                    {file.name}
                  </Text>
                  <Text size="caption" tone="secondary" truncate>
                    {Math.max(1, Math.round(file.size / 1024))} KB · SHA-256 {file.sha256.slice(0, 8)}
                  </Text>
                </VStack>
              </HStack>
            ))
          )}
        </CardSection>

        <CardSection
          title={tr('Bookmarks & Notes')}
          footer={tr('Saved messages and personal annotations for quick reference (N14).')}
        >
          {bookmarks.length === 0 ? (
            <Text tone="secondary">{tr('Right-click any message in the chat and choose Bookmark to save it here.')}</Text>
          ) : (
            bookmarks.map((bm) => (
              <VStack key={bm.id} gap={6}>
                <HStack align="center" gap={10}>
                  <Icon name="bookmark.fill" size={13} color="var(--accent)" />
                  <VStack gap={2} grow>
                    <Text weight="medium" truncate>
                      {bm.preview || tr('(Message)')}
                    </Text>
                    {bm.note && editingNoteId !== bm.entryId ? (
                      <Text size="caption" tone="secondary">
                        {bm.note}
                      </Text>
                    ) : null}
                  </VStack>
                  <IconButton
                    title={tr('Edit note')}
                    icon="pencil"
                    onClick={() => {
                      setEditingNoteId(bm.entryId)
                      setNoteDraft(bm.note ?? '')
                    }}
                  />
                  <IconButton
                    title={tr('Remove bookmark')}
                    icon="trash"
                    onClick={async () => {
                      await store.deleteBookmark(botId, bm.entryId)
                      await refresh()
                    }}
                  />
                </HStack>
                {editingNoteId === bm.entryId ? (
                  <HStack gap={6} align="center">
                    <input
                      className="field-box"
                      value={noteDraft}
                      placeholder={tr('Add a personal note…')}
                      onChange={(e) => setNoteDraft(e.target.value)}
                    />
                    <Button
                      kind="primary"
                      onClick={async () => {
                        await store.saveBookmark(botId, bm.entryId, bm.preview, noteDraft, bm.author, bm.threadId ?? null)
                        setEditingNoteId(null)
                        await refresh()
                      }}
                    >
                      {tr('Save')}
                    </Button>
                  </HStack>
                ) : null}
              </VStack>
            ))
          )}
        </CardSection>
      </div>

      <BotAppPanel
        app={activeApp}
        onClose={() => setActiveApp(null)}
        onCreateShareLink={async (slug, label, mode) => {
          const shared = await store.createAppLink(botId, slug, mode, label)
          toasts.push(tr('Share link created'), 'success')
          await refresh()
          return shared
        }}
        onRevokeShareLink={async (slug, token) => {
          await store.revokeAppLink(botId, slug, token)
          toasts.push(tr('Share link revoked'), 'info')
          await refresh()
        }}
      />
      <ToastContainer toasts={toasts.toasts} onDismiss={toasts.dismiss} />
    </div>
  )
}
