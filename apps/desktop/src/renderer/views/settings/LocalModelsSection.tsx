import { useEffect } from 'react'
import { Button, CardSection } from '../../components/Controls'
import { HStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useModel } from '../../lib/observable'
import type { MaintenanceStore } from '../../store/maintenance-store'

/**
 * A local Ollama, if one is running: its models and the embedding model semantic search will use.
 * The chat itself still runs through an agent harness; this is the local-model foundation.
 */
export function LocalModelsSection({ store, online }: { store: MaintenanceStore; online: boolean }) {
  const model = useModel(store)
  useEffect(() => {
    if (online) void store.loadLocalModels()
  }, [online, store])
  const status = model.localModels
  return (
    <CardSection title="Local models (Ollama)"
      footer="Semantic workspace search reranks results with this Ollama's embeddings when it is running; without it, search stays lexical. Set CODYNC_OLLAMA_URL and CODYNC_EMBED_MODEL to point at another install or model.">
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
            {status.models.length ? `Models: ${status.models.join(', ')}` : 'No models are installed yet.'}
          </Text>
        </>
      ) : null}
      {store.error ? <Text size="caption" tone="danger">{store.error}</Text> : null}
    </CardSection>
  )
}
