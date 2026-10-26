import { useState } from 'react'
import { Button, CardSection, Spinner } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { Dialog } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import type { MaintenanceStore } from '../../store/maintenance-store'

/** `doctor`: what's wrong with the bots' setup, and a way to clear the failed routine runs nobody is waiting on. */
export function BotCheckSection({ store, online }: { store: MaintenanceStore; online: boolean }) {
  const [confirmClear, setConfirmClear] = useState(false)
  const report = store.report
  return (
    <CardSection title="Bot check" footer="Finds bots that can't run as set up: a missing folder, no agent, a group member that was deleted.">
      <HStack gap={8}>
        <Button kind="secondary" disabled={!online || store.busy} onClick={() => void store.check()}>
          {store.busy ? <Spinner size={12} /> : null}Check bots
        </Button>
        <Button kind="secondary" disabled={!online || store.busy} onClick={() => setConfirmClear(true)}>Clear failed routine runs</Button>
      </HStack>
      {report ? (
        <Text tone={report.errors ? 'attention' : 'secondary'}>
          {report.errors ? `${report.errors} of ${report.checked} bots can't run as set up.` : `${report.checked} bots checked. Nothing to fix.`}
        </Text>
      ) : null}
      {report?.findings.map((f) => (
        <VStack key={`${f.botId}:${f.problem}`} gap={2} className="settings-row">
          <HStack gap={6}>
            <Icon name={f.severity === 'error' ? 'xmark.octagon.fill' : 'exclamationmark.triangle.fill'} size={13}
              color={f.severity === 'error' ? 'var(--danger)' : 'var(--warning)'} />
            <Text>{f.message}</Text>
          </HStack>
        </VStack>
      ))}
      {store.error ? <Text size="footnote" tone="danger">{store.error}</Text> : null}
      {store.notice ? <Text as="div" role="status" size="footnote" tone="secondary">{store.notice}</Text> : null}
      <Dialog open={confirmClear} title="Clear failed routine runs?"
        message="Failed runs you can no longer retry are removed from every bot's routine history. Runs waiting to retry stay."
        actions={[{ title: 'Clear', action: () => { setConfirmClear(false); void store.clearFailedRuns() } }]}
        onClose={() => setConfirmClear(false)} />
    </CardSection>
  )
}
