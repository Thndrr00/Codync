import { useEffect, useState } from 'react'
import type { BotInsights } from '@shared/bot-insights'
import { CardSection, ValueRow } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'

/** The bot's routines, model and daily use, read from the host's `botInsights` method. */
export function BotInsightsCard({ botId }: { botId: string }) {
  const store = useStore()
  const tr = useTr()
  const [insights, setInsights] = useState<BotInsights | null>(null)
  const [error, setError] = useState(false)

  useEffect(() => {
    let live = true
    setInsights(null)
    setError(false)
    void (async () => {
      try {
        const answer = await (await store.ready()).call<BotInsights>('botInsights', { botId })
        if (live) setInsights(answer)
      } catch {
        if (live) setError(true)
      }
    })()
    return () => { live = false }
  }, [botId, store])

  if (error) return <CardSection title={tr('Insights')}><Text size="footnote" tone="secondary">{tr('Could not load insights.')}</Text></CardSection>
  if (!insights) return <CardSection title={tr('Insights')}><Text size="footnote" tone="secondary">{tr('Loading…')}</Text></CardSection>

  const { usage, skills } = insights
  return (
    <VStack gap={12}>
      <CardSection title={tr('Model')}>
        <ValueRow label={tr('Agent')}>{insights.provider ?? tr('Unknown')}</ValueRow>
        {insights.model ? <ValueRow label={tr('Model')}>{insights.model}</ValueRow> : null}
      </CardSection>

      <CardSection title={tr('Today')}>
        <ValueRow label={tr('Tokens')}>
          {usage.limit === null ? usage.usedText : `${usage.usedText} / ${usage.limit}`}
        </ValueRow>
        <ValueRow label={tr('Skills')}>{String(skills.total)}</ValueRow>
      </CardSection>

      {insights.recentWork.length ? (
        <CardSection title={tr('Recent work')}>
          {insights.recentWork.map((line, i) => (
            <Text key={i} size="footnote" tone="secondary">{line}</Text>
          ))}
        </CardSection>
      ) : null}

      <CardSection title={tr('Routines')}>
        {insights.routines.length === 0 ? (
          <Text size="footnote" tone="secondary">{tr('No routines yet.')}</Text>
        ) : (
          insights.routines.map((routine) => (
            <VStack key={routine.id} gap={2}>
              <HStack gap={6}>
                <Text size="subheadline" weight="semibold">{routine.name}</Text>
                {!routine.enabled ? <Text size="footnote" tone="secondary">{tr('Paused')}</Text> : null}
              </HStack>
              {routine.schedules.length ? (
                <Text size="footnote" tone="secondary">{routine.schedules.join(' · ')}</Text>
              ) : null}
              {routine.nextIn ? (
                <Text size="footnote" tone="tertiary">{tr('Next in {time}').replace('{time}', routine.nextIn)}</Text>
              ) : null}
            </VStack>
          ))
        )}
      </CardSection>
    </VStack>
  )
}
