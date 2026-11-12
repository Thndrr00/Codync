import { useState } from 'react'
import { Button, CardSection } from '../../components/Controls'
import { HStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useModel } from '../../lib/observable'
import type { MaintenanceStore } from '../../store/maintenance-store'
import { Field } from '../marketplace/pieces'

/** How long messages are kept: preview what a cutoff would delete, then delete it on purpose. */
export function RetentionSection({ store, online }: { store: MaintenanceStore; online: boolean }) {
  useModel(store)
  const [days, setDays] = useState('90')
  const value = Number.parseInt(days.replace(/[\s,]/g, ''), 10)
  const valid = Number.isFinite(value) && value >= 1 && value <= 3650
  const preview = store.retention
  return (
    <CardSection title="Message retention"
      footer="Counts every message, notice and tool step in this computer's chats. A preview changes nothing; deleting is permanent, so keep a backup of the database if you may need it. Documents, memories and routines are not touched.">
      <Field label="Keep messages for (days)" value={days} onChange={setDays} placeholder="For example 90" />
      <HStack gap={8}>
        <Button kind="secondary" disabled={!online || store.busy || !valid} onClick={() => void store.previewRetention(value)}>
          Preview
        </Button>
        <Button disabled={!online || store.busy || !preview || preview.days !== value} onClick={() => void store.pruneRetention(value)}>
          Delete older messages
        </Button>
      </HStack>
      {preview ? (
        <Text size="caption" tone="secondary">
          {preview.messages === 0
            ? 'Nothing is that old.'
            : `${preview.messages.toLocaleString()} messages older than ${preview.days} days would be deleted.`}
        </Text>
      ) : null}
    </CardSection>
  )
}
