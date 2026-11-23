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

  const { usage, skills, activity } = insights
  const toneOf = (tone: string): 'text' | 'secondary' | 'tertiary' | 'danger' | 'attention' | 'accent' | 'added' | 'removed' => {
    if (tone === 'danger' || tone === 'error') return 'danger'
    if (tone === 'accent') return 'accent'
    if (tone === 'good') return 'added'
    if (tone === 'warn') return 'attention'
    if (tone === 'tertiary') return 'tertiary'
    return 'secondary'
  }
  return (
    <VStack gap={12}>
      {insights.description ? (
        <CardSection title={tr('About')}>
          <Text size="footnote" tone="secondary">{insights.description}</Text>
        </CardSection>
      ) : null}

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

      <CardSection title={tr('Activity')}>
        <Text size="footnote" tone="secondary">{activity.headline}</Text>
        <ValueRow label={tr('Steps')}>{String(activity.steps)}</ValueRow>
        {activity.errors ? (
          <Text size="footnote" tone="danger">
            {tr('{count} failed').replace('{count}', String(activity.errors))}
          </Text>
        ) : null}
        {activity.groups.length ? (
          <HStack gap={8}>
            {activity.groups.slice(0, 6).map((group) => (
              <Text key={group.kind} size="footnote" tone={toneOf(group.tone)}>
                {`${group.count} ${group.label}`}
              </Text>
            ))}
          </HStack>
        ) : null}
        {activity.days.length ? (
          <VStack gap={2}>
            {activity.days.slice(-5).map((day) => (
              <HStack key={day.day} gap={6}>
                <Text size="footnote" tone="tertiary">{day.label}</Text>
                <Text size="footnote" tone="accent">{'█'.repeat(Math.max(1, day.width))}</Text>
                <Text size="footnote" tone="secondary">{String(day.count)}</Text>
              </HStack>
            ))}
          </VStack>
        ) : null}
        {activity.items.length ? (
          <VStack gap={2}>
            {activity.items.slice(0, 6).map((item, i) => (
              <VStack key={`${item.text}-${i}`} gap={0}>
                <Text size="footnote" tone={toneOf(item.status === 'error' ? 'danger' : item.tone)}>{item.text}</Text>
                {item.detail ? (
                  <Text size="footnote" tone="tertiary">{item.detail}</Text>
                ) : null}
              </VStack>
            ))}
          </VStack>
        ) : null}
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
