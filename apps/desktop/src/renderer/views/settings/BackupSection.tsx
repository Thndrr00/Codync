import { Button, CardSection } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useModel } from '../../lib/observable'
import type { MaintenanceStore } from '../../store/maintenance-store'

/**
 * A copy of the whole database for safekeeping: bots, chats, documents, routines, board and memory.
 * It goes under the host's data folder (backups/), and the original stays where it was.
 */
export function BackupSection({ store, online }: { store: MaintenanceStore; online: boolean }) {
  const model = useModel(store)
  return (
    <CardSection title="Backup"
      footer="Writes one consistent copy of the database, with everything this computer keeps, under its data folder in backups/. Copy it somewhere else to keep it safe; restoring is done by stopping the host and putting the file back in place.">
      <HStack gap={8} align="center" wrap>
        <Button kind="secondary" disabled={!online || store.busy} onClick={() => void store.backupDatabase()}>
          {store.busy ? 'Backing up…' : 'Back up now'}
        </Button>
        {model.backupPath ? (
          <Button kind="secondary" disabled={!online} onClick={() => void window.codync.app.copy(model.backupPath ?? '')}>Copy path</Button>
        ) : null}
      </HStack>
      {model.backupPath ? (
        <VStack gap={2}>
          <Text size="caption" tone="secondary">Last backup</Text>
          <Text size="caption" tone="tertiary" className="scheduled-text">{model.backupPath}</Text>
        </VStack>
      ) : null}
      {store.error ? <Text size="caption" tone="danger">{store.error}</Text> : null}
    </CardSection>
  )
}
