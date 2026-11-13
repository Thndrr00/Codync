import { useEffect, useState } from 'react'
import { Button, CardSection, ChoicePicker } from '../../components/Controls'
import { HStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useModel } from '../../lib/observable'
import type { MaintenanceStore } from '../../store/maintenance-store'

/**
 * A local Ollama, if one is running: its models and the embedding model semantic search will use.
 * One of its chat models can become a bot (codync-host's local ACP agent); it has no tools.
 */
export function LocalModelsSection({ store, online }: { store: MaintenanceStore; online: boolean }) {
  const model = useModel(store)
  const [chosen, setChosen] = useState('')
  useEffect(() => {
    if (online) void store.loadLocalModels()
  }, [online, store])
  const status = model.localModels
  const models = status?.models ?? []
  const selection = models.includes(chosen) ? chosen : models[0] ?? ''
  return (
    <CardSection title="Local models (Ollama)"
      footer="Semantic workspace search reranks results with this Ollama's embeddings when it is running; without it, search stays lexical. A local bot answers with the chosen model, without tools. Set CODYNC_OLLAMA_URL and CODYNC_EMBED_MODEL to point at another install or model.">
      <HStack gap={8} align="center" wrap>
        <Text size="caption" tone={status?.available ? 'secondary' : 'tertiary'}>
          {status ? (status.available ? `Running at ${status.url}` : `Not running at ${status.url}`) : 'Checking…'}
        </Text>
        <Button kind="secondary" disabled={!online || store.busy} onClick={() => void store.loadLocalModels()}>Check again</Button>
      </HStack>
      {status?.available ? (
        <>
          <Text size="caption" tone="secondary">Embedding model for semantic search: {status.embeddingModel ?? 'nomic-embed-text'}</Text>
          <Text size="caption" tone="secondary">
            {models.length ? `Models: ${models.join(', ')}` : 'No models are installed yet.'}
          </Text>
          {models.length ? (
            <HStack gap={8} align="center" wrap>
              <ChoicePicker selection={selection} options={models.map((name) => ({ id: name, label: name }))} onChange={setChosen} />
              <Button kind="secondary" disabled={!online || store.busy || !selection} onClick={() => void store.createLocalBot(selection)}>Create a local bot</Button>
            </HStack>
          ) : null}
        </>
      ) : null}
      {store.notice ? <Text size="caption" tone="secondary">{store.notice}</Text> : null}
      {store.error ? <Text size="caption" tone="danger">{store.error}</Text> : null}
    </CardSection>
  )
}
