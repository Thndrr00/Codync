import { useState, type ReactNode } from 'react'
import { Button, CardSection, Spinner, ValueRow } from '../../components/Controls'
import { FolderPicker } from '../../components/FolderPicker'
import { Icon } from '../../components/Icon'
import { Sheet } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'
import { useDocsIndex, type DocsIndexStatus } from '../../store/use-docs-index'
import { byteCount } from '../thread/Attachments'

/** What the index holds: the folder it reads, how much it found, and whether a limit cut it short. */
function indexRows(status: DocsIndexStatus, busy: boolean, tr: (text: string) => string): ReactNode[] {
  return [
    <ValueRow key="folder" label="Folder">
      <Text truncate title={status.folder}>{status.folder}</Text>
    </ValueRow>,
    <ValueRow key="files" label="Files">
      {busy ? <Spinner /> : <Text>{status.files.toLocaleString()}</Text>}
    </ValueRow>,
    <ValueRow key="size" label="Size">
      {busy ? <Spinner /> : <Text>{byteCount(status.bytes)}</Text>}
    </ValueRow>,
    status.truncated ? (
      <HStack key="limit" gap={8} align="baseline">
        <Icon name="exclamationmark.triangle.fill" size={12} color="var(--attention)" />
        <Text size="caption" tone="attention">
          {tr('The index stopped at its limit of 4,000 files or 24 MB, so a search may miss some files.')}
        </Text>
      </HStack>
    ) : null,
  ]
}

/**
 * A bot's documentation and code search (the host's `codync-docs` server): the folder the computer
 * indexes, what it holds, and the choice between a folder of its own and the bot's working folder.
 */
export function DocsSearchSection({ botId, workingFolder }: { botId: string; workingFolder: string }) {
  const tr = useTr()
  const store = useStore()
  const docs = useDocsIndex(botId)
  const [choosing, setChoosing] = useState(false)
  const online = store.connection.kind === 'online'
  const status = docs.status
  return (
    <>
      <CardSection
        title="Documentation search"
        footer="Your bot searches this folder for documentation and code and cites the file and line. Until you choose another folder, it is the bot's working folder. Generated folders and what .gitignore lists are skipped."
      >
        {status ? indexRows(status, docs.busy, tr) : null}
        {!status && !docs.error && online ? (
          <HStack gap={8}>
            <Spinner />
            <Text tone="secondary">{tr('Reading the index…')}</Text>
          </HStack>
        ) : null}
        <VStack gap={10}>
          <HStack gap={8} wrap>
            <Button kind="secondary" disabled={!online || docs.busy} onClick={() => setChoosing(true)}>{tr('Choose folder')}</Button>
            <Button kind="secondary" disabled={!online || docs.busy} onClick={() => docs.choose('')}>{tr('Use working folder')}</Button>
          </HStack>
          {docs.error ? (
            <HStack gap={8} wrap>
              <Text size="caption" tone="danger" role="alert">{docs.error}</Text>
              {status ? null : <Button kind="secondary" disabled={!online || docs.loading} onClick={docs.reload}>{tr('Try again')}</Button>}
            </HStack>
          ) : null}
          {!online ? <Text size="caption" tone="secondary">{tr('Reconnect to this computer to change the indexed folder.')}</Text> : null}
        </VStack>
      </CardSection>
      <Sheet open={choosing} onClose={() => setChoosing(false)} width={460} height={520}>
        <FolderPicker
          start={status?.folder || workingFolder || store.hello?.home || null}
          onPick={(path) => {
            docs.choose(path)
            setChoosing(false)
          }}
        />
      </Sheet>
    </>
  )
}
