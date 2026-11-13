import { useState } from 'react'
import type { Bot } from '@shared/models'
import { Button, CardSection, ChoicePicker, Spinner } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useModel } from '../../lib/observable'
import type { MaintenanceStore } from '../../store/maintenance-store'
import { Field } from '../marketplace/pieces'

/**
 * The same prompt through two (or three) bots, each in a throwaway session: nothing is posted into
 * their chats and nothing is remembered. Useful to compare two models, or two agents with different
 * instructions, before choosing which one keeps a job.
 */
export function ModelCompareSection({ store, bots, online }: {
  store: MaintenanceStore
  bots: Bot[]
  online: boolean
}) {
  const model = useModel(store)
  const choices = bots.map((bot) => ({ id: bot.id, label: `${bot.name} · ${bot.backend}${bot.model ? ` (${bot.model})` : ''}` }))
  const [first, setFirst] = useState('')
  const [second, setSecond] = useState('')
  const [prompt, setPrompt] = useState('')
  const a = choices.find((c) => c.id === first) ? first : (choices[0]?.id ?? '')
  const b = choices.find((c) => c.id === second) && second !== a ? second : (choices.find((c) => c.id !== a)?.id ?? '')
  const ready = online && a && b && prompt.trim().length > 0 && !store.busy

  return (
    <CardSection title="Compare models"
      footer="Each bot answers in a throwaway session: the prompt is not added to its chat and nothing is remembered. Only bots on this computer are compared.">
      {choices.length < 2 ? (
        <Text size="caption" tone="secondary">Two bots are needed to compare; add another agent first.</Text>
      ) : (
        <>
          <HStack gap={10} align="end">
            <VStack gap={4} grow>
              <Text size="caption" tone="secondary">First</Text>
              <ChoicePicker selection={a} options={choices} onChange={setFirst} />
            </VStack>
            <VStack gap={4} grow>
              <Text size="caption" tone="secondary">Second</Text>
              <ChoicePicker selection={b} options={choices.filter((c) => c.id !== a)} onChange={setSecond} />
            </VStack>
          </HStack>
          <Field label="Prompt" value={prompt} onChange={setPrompt} lines={[3, 6]} placeholder="The same question for both" />
          <HStack gap={8}>
            <Button disabled={!ready} onClick={() => void store.compareModels([a, b], prompt.trim())}>
              {store.busy ? <Spinner size={12} /> : null}
              {store.busy ? 'Comparing…' : 'Compare'}
            </Button>
          </HStack>
        </>
      )}
      {model.compare.length > 0 ? (
        <VStack gap={10} className="compare-grid">
          {model.compare.map((result) => (
            <VStack key={result.botId} gap={4} className="compare-card">
              <HStack gap={6} align="center">
                <Text weight="semibold" grow truncate>{result.name}</Text>
                <Text size="caption" tone="tertiary">{result.model ?? result.backend} · {(result.ms / 1000).toFixed(1)} s</Text>
              </HStack>
              {result.error ? (
                <Text size="caption" tone="danger">{result.error}</Text>
              ) : (
                <Text size="callout" className="compare-answer">{result.text}</Text>
              )}
            </VStack>
          ))}
        </VStack>
      ) : null}
      {store.error ? <Text size="caption" tone="danger">{store.error}</Text> : null}
    </CardSection>
  )
}
