import { Button, CardSection } from '../../components/Controls'
import { Text } from '../../components/Text'
import { useModel } from '../../lib/observable'
import type { MaintenanceStore } from '../../store/maintenance-store'

/** One JSON file with every bot and its documents, copied to the clipboard to keep or move. */
export function WorkspaceExportSection({ store, online }: { store: MaintenanceStore; online: boolean }) {
  useModel(store)
  return (
    <CardSection title="Workspace export"
      footer="Includes each bot's setup, skills, connectors and Markdown documents; its routines; the messages still scheduled; and the newest 300 messages of each chat. API keys, sign-ins and connector secrets are not included. Paste it into a file to keep it.">
      <Button kind="secondary" disabled={!online || store.busy} onClick={() => void store.exportWorkspace()}>
        Copy workspace export
      </Button>
      {store.notice ? <Text size="caption" tone="secondary">{store.notice}</Text> : null}
    </CardSection>
  )
}
