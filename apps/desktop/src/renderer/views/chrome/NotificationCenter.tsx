import { useEffect, useState, useSyncExternalStore } from 'react'
import { IconButton } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { ModalHeader, Sheet, SheetPage } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useModels, useModel } from '../../lib/observable'
import { useTr } from '../../lib/use-language'
import { useApp } from '../../store/context'
import { collectNotices, type Notice, type NoticeKind } from './notifications'
import './notification-center.css'

/**
 * The read keys are shared by the two bells (the open sidebar's and the collapsed rail's), so
 * they show the same count. This is window state, not a device pref: a restart starts unread.
 */
const seen = new Set<string>()
const seenListeners = new Set<() => void>()
let seenVersion = 0
const subscribeSeen = (listener: () => void) => {
  seenListeners.add(listener)
  return () => void seenListeners.delete(listener)
}
const getSeenVersion = () => seenVersion
function markSeen(keys: string[]) {
  let added = false
  for (const key of keys) if (!seen.has(key)) { seen.add(key); added = true }
  if (added) {
    seenVersion++
    for (const listener of seenListeners) listener()
  }
}

const KIND_ICON: Record<NoticeKind, string> = {
  question: 'bubble.left.and.bubble.right',
  approval: 'person.crop.circle.badge.checkmark',
  needsInput: 'exclamationmark.circle',
  error: 'exclamationmark.triangle',
}

/**
 * The top-row bell and the notification center it opens. The list is derived from the live
 * stores on every render (no polling); opening it marks the items currently shown as read,
 * and the badge counts the ones that arrived since. The shared seen set survives the sheet
 * closing but is window-local, like the rest of the chrome.
 */
export function NotificationCenter({ size = 32 }: { size?: number }) {
  const tr = useTr()
  const app = useApp()
  useModel(app)
  useModels([...app.stores.values()])
  useSyncExternalStore(subscribeSeen, getSeenVersion)
  const [open, setOpen] = useState(false)
  const notices = collectNotices({
    roster: app.roster.map(({ ref, bot, store }) => ({ ref, bot, entries: store.allEntries(bot.id) })),
    approvals: app.approvals.map((a) => ({ id: a.id, deviceName: a.request.deviceName, createdAt: a.request.createdAt ?? null })),
  })
  const keyList = notices.map((n) => n.key).join('\n')
  // New arrivals while the sheet is open are visible, so they count as read too.
  useEffect(() => {
    if (open) markSeen(notices.map((notice) => notice.key))
    // `keyList` is the stable signature of `notices`; re-running on each render would loop.
  }, [open, keyList])

  const unread = notices.filter((notice) => !seen.has(notice.key)).length
  const label = unread ? `${tr('Notifications')} (${unread})` : tr('Notifications')
  const activate = (notice: Notice) => {
    if (notice.kind === 'approval') app.reviewApprovals()
    else if (notice.ref) app.select(notice.ref)
    setOpen(false)
  }

  return (
    <>
      <span className="notification-bell">
        <IconButton title={label} icon="bell.badge" size={size} className="sidebar-header-action" onClick={() => setOpen(true)} />
        {unread > 0 ? <span className="notification-badge" aria-hidden>{unread > 9 ? '9+' : unread}</span> : null}
      </span>
      <Sheet open={open} onClose={() => setOpen(false)} width={480} maxHeight={560}>
        <SheetPage>
          <ModalHeader icon="bell.badge" title={tr('Notifications')} subtitle={tr('What needs your attention across your bots and computers.')} />
          <VStack gap={6} className="notification-list">
            {notices.length === 0 ? (
              <Text size="callout" tone="secondary" className="notification-empty">{tr('You’re all caught up.')}</Text>
            ) : notices.map((notice) => (
              <button key={notice.key} type="button" className="notification-row" onClick={() => activate(notice)}>
                <span className="notification-icon" aria-hidden><Icon name={KIND_ICON[notice.kind]} size={15} /></span>
                <VStack gap={1} grow>
                  <Text size="callout" weight="medium" truncate>{tr(notice.title)}</Text>
                  <Text size="caption" tone="secondary" truncate>{notice.detail}</Text>
                </VStack>
                <Icon name="chevron.right" size={10} weight="semibold" color="var(--tertiary)" />
              </button>
            ))}
          </VStack>
          <HStack className="notification-footer">
            <Text size="caption" tone="tertiary">{tr('Opens the chat or approval that needs you.')}</Text>
          </HStack>
        </SheetPage>
      </Sheet>
    </>
  )
}
