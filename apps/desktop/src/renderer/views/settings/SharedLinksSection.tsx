import { useEffect } from 'react'
import { Button, CardSection } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useModel } from '../../lib/observable'
import type { MaintenanceStore } from '../../store/maintenance-store'
import { relativeTime } from '../thread/chat-items'

const size = (bytes: number) => `${Math.max(1, Math.round(bytes / 1024)).toLocaleString()} KB`

/**
 * The read-only pages this computer serves by link: a chat transcript or a document, for a week or
 * until revoked. The link is the only key; revoking it deletes the page at once.
 */
export function SharedLinksSection({ store, online }: { store: MaintenanceStore; online: boolean }) {
  const model = useModel(store)
  useEffect(() => {
    if (online) void store.loadLinks()
  }, [online, store])
  return (
    <CardSection title="Read-only links"
      footer="A link shows one page to anyone who has it, without a sign-in. It expires a week after it is made and can be revoked here at any time.">
      {model.links.length === 0 ? (
        <Text size="caption" tone="secondary">No links are being served.</Text>
      ) : (
        model.links.map((link) => (
          <VStack key={link.token} gap={4} className="scheduled-row">
            <HStack gap={8} align="center">
              <Text weight="semibold" grow truncate>{link.title}</Text>
              <Text size="caption" tone="secondary">{size(link.bytes)}</Text>
            </HStack>
            <Text size="caption" tone="secondary">Made {relativeTime.day(link.createdAt)} · expires {relativeTime.day(link.expiresAt)}</Text>
            <HStack gap={6}>
              <Button kind="secondary" disabled={!online || store.busy} onClick={() => void window.codync.app.copy(link.url)}>Copy link</Button>
              <Button kind="secondary" disabled={!online || store.busy} onClick={() => void store.revokeLink(link.token)}>Revoke</Button>
            </HStack>
          </VStack>
        ))
      )}
      {store.notice ? <Text size="caption" tone="secondary">{store.notice}</Text> : null}
      {store.error ? <Text size="caption" tone="danger">{store.error}</Text> : null}
      <Button kind="secondary" disabled={!online || store.busy} onClick={() => void store.loadLinks()}>Reload</Button>
    </CardSection>
  )
}
