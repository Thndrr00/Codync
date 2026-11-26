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

  const { usage, skills, activity, health, profile, week, routineSummary, library, trust, resilience, identity, connectors, tuning, memory, workspace, session, advice, digest, timezone, approvals, failures, conversation, agent, fresh, prompts, directory, blocks, access, dictation, liveness, context, opendots, openbot, agenteschat, textfmt } = insights
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
      <CardSection title={tr('Text hygiene')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(textfmt.tone)}>{textfmt.headline}</Text>
        <ValueRow label={tr('Handle')}>{textfmt.handle}</ValueRow>
        <ValueRow label={tr('Invalid emails')}>{String(textfmt.invalidEmails)}</ValueRow>
        <ValueRow label={tr('Hidden marks')}>{String(textfmt.hidden)}</ValueRow>
      </CardSection>

      <CardSection title={tr('Context budget')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(agenteschat.windowTone)}>{agenteschat.headline}</Text>
        <ValueRow label={tr('Window')}>{agenteschat.window}</ValueRow>
        <ValueRow label={tr('Mode')}>{agenteschat.auto}</ValueRow>
      </CardSection>

      <CardSection title={tr('Run model')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(openbot.tone)}>{openbot.headline}</Text>
        <ValueRow label={tr('Effort')}>{openbot.effort}</ValueRow>
        <ValueRow label={tr('Route')}>{openbot.route}</ValueRow>
      </CardSection>

      <CardSection title={tr('Learning')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(opendots.learningTone)}>{opendots.headline}</Text>
        <ValueRow label={tr('Container')}>{opendots.container}</ValueRow>
        <ValueRow label={tr('Internal messages')}>{String(opendots.internal)}</ValueRow>
      </CardSection>

      <CardSection title={tr('Context window')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(context.tone)}>{context.headline}</Text>
        <ValueRow label={tr('Page states')}>{context.page}</ValueRow>
        <ValueRow label={tr('Screenshots')}>{context.screenshots}</ValueRow>
        <Text size="footnote" tone="tertiary">{context.pageNote}</Text>
      </CardSection>

      <CardSection title={tr('Liveness')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(liveness.tone)}>{liveness.headline}</Text>
        <ValueRow label={tr('State')}>{liveness.state}</ValueRow>
        <ValueRow label={tr('Delivery')}>{liveness.outcome}</ValueRow>
      </CardSection>

      <CardSection title={tr('Voice dictation')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(dictation.tone)}>{dictation.headline}</Text>
        <ValueRow label={tr('Engine')}>{dictation.engine}</ValueRow>
        <ValueRow label={tr('Model')}>{dictation.model}</ValueRow>
        <ValueRow label={tr('Language')}>{dictation.language}</ValueRow>
      </CardSection>

      <CardSection title={tr('Public access')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(access.modeTone)}>{access.mode}</Text>
        <ValueRow label={tr('Routing')}>{access.routing}</ValueRow>
        {access.collections.length ? (
          <VStack gap={2}>
            {access.collections.map((item) => (
              <HStack key={item.name} gap={6}>
                <Text size="footnote" tone={item.valid ? 'text' : 'danger'}>{item.name}</Text>
              </HStack>
            ))}
          </VStack>
        ) : null}
      </CardSection>

      <CardSection title={tr('Last message')}>
        <Text size="subheadline" weight="semibold">{blocks.headline}</Text>
        <Text size="footnote" tone="tertiary">{blocks.origin}</Text>
        {blocks.blocks.map((item, i) => (
          <VStack key={`${item.kind}-${i}`} gap={0}>
            <Text size="footnote" tone={toneOf(item.tone)}>{`${item.says ? '●' : '○'} ${item.text}`}</Text>
          </VStack>
        ))}
      </CardSection>

      <CardSection title={tr('Directory')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(directory.tone)}>{directory.headline}</Text>
        <Text size="footnote" tone="accent">{directory.bar}</Text>
        <ValueRow label={tr('Directory')}>{directory.directory}</ValueRow>
        <ValueRow label={tr('Integrations')}>{String(directory.integrations.length)}</ValueRow>
        {directory.integrations.map((item) => (
          <Text key={item.id} size="footnote" tone="secondary">{`${item.name} · ${item.header}`}</Text>
        ))}
      </CardSection>

      <CardSection title={tr('Prompt style')}>
        <Text size="subheadline" weight="semibold">{prompts.model}</Text>
        <Text size="footnote" tone="secondary">{prompts.price}</Text>
        <Text size="footnote" tone="tertiary">{`${prompts.context} · ${prompts.zone} · ${prompts.length}`}</Text>
        {prompts.lines.map((line, i) => (
          <Text key={i} size="footnote" tone="secondary">{line}</Text>
        ))}
      </CardSection>

      <CardSection title={tr('Latest from bdots')}>
        <Text size="subheadline" weight="semibold">{fresh.agent}</Text>
        {fresh.summary.map((row) => (
          <HStack key={row.key} gap={6}>
            <Text size="footnote" tone="tertiary">{row.key}</Text>
            <Text size="footnote" tone="secondary">{row.value}</Text>
          </HStack>
        ))}
        <Text size="footnote" tone="tertiary">{`${fresh.titles.exec} · ${fresh.titles.write}`}</Text>
      </CardSection>

      <CardSection title={tr('Agent profile')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(agent.tone)}>{agent.label}</Text>
        <Text size="footnote" tone="secondary">{agent.headline}</Text>
        {agent.lines.map((item) => (
          <HStack key={item.id} gap={6}>
            <Text size="footnote" tone={toneOf(item.stateTone)}>{item.set ? '●' : '○'}</Text>
            <Text size="footnote" tone={toneOf(item.stateTone)}>{`${item.label}: ${item.detail}`}</Text>
          </HStack>
        ))}
      </CardSection>

      <CardSection title={tr('Conversation')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(conversation.tone)}>{conversation.headline}</Text>
        <Text size="footnote" tone="secondary">{conversation.line}</Text>
        {conversation.kinds.map((kind) => (
          <HStack key={kind.kind} gap={6}>
            <Text size="footnote" tone={toneOf(kind.tone)}>{kind.label}</Text>
            <Text size="footnote" tone="accent">{kind.bar}</Text>
            <Text size="footnote" tone="secondary">{String(kind.count)}</Text>
          </HStack>
        ))}
      </CardSection>

      {failures.total !== '0' ? (
        <CardSection title={tr('Recent failures')}>
          <Text size="subheadline" weight="semibold" tone={toneOf(failures.tone)}>{failures.headline}</Text>
          {failures.categories.map((cat) => (
            <HStack key={cat.kind} gap={6}>
              <Text size="footnote" tone={toneOf(cat.tone)}>{cat.kind}</Text>
              <Text size="footnote" tone="accent">{cat.bar}</Text>
              <Text size="footnote" tone="secondary">{String(cat.count)}</Text>
            </HStack>
          ))}
          {failures.items.map((item, i) => (
            <VStack key={`${item.name}-${i}`} gap={0}>
              <Text size="footnote" tone={toneOf(item.tone)}>{item.name}</Text>
              <Text size="footnote" tone="tertiary">{`${item.reason} · ${item.age}`}</Text>
            </VStack>
          ))}
        </CardSection>
      ) : null}

      <CardSection title={tr('Approvals')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(approvals.tone)}>{approvals.headline}</Text>
        <Text size="footnote" tone="accent">{approvals.bar}</Text>
        {approvals.items.map((item) => (
          <VStack key={item.id} gap={0}>
            <Text size="footnote" tone={toneOf(item.tone)}>{item.title}</Text>
            <Text size="footnote" tone="tertiary">{`${item.detail} · ${item.age}`}</Text>
          </VStack>
        ))}
      </CardSection>

      <CardSection title={tr('Time zones')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(timezone.tone)}>{timezone.headline}</Text>
        <ValueRow label={tr('This computer')}>{timezone.local}</ValueRow>
        {timezone.all.map((zone) => (
          <HStack key={zone.name} gap={6}>
            <Text size="footnote" tone={toneOf(zone.tone)}>{zone.label}</Text>
            <Text size="footnote" tone="accent">{zone.bar}</Text>
            <Text size="footnote" tone="secondary">{String(zone.count)}</Text>
          </HStack>
        ))}
      </CardSection>

      <CardSection title={tr('At a glance')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(digest.tone)}>{digest.label}</Text>
        <Text size="footnote" tone="secondary">{digest.headline}</Text>
        <Text size="footnote" tone="tertiary">{digest.oneLine}</Text>
        {digest.stats.map((item) => (
          <HStack key={item.id} gap={6}>
            <Text size="footnote" tone={toneOf(item.tone)}>{item.label}</Text>
            <Text size="footnote" tone="accent">{item.value}</Text>
          </HStack>
        ))}
      </CardSection>

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

      <CardSection title={tr('Suggestions')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(advice.tone)}>{advice.label}</Text>
        <Text size="footnote" tone="secondary">{advice.headline}</Text>
        {advice.items.map((item) => (
          <VStack key={item.id} gap={0}>
            <Text size="footnote" tone={toneOf(item.priorityTone)}>{`${item.priorityLabel}: ${item.title}`}</Text>
            <Text size="footnote" tone="tertiary">{item.detail}</Text>
          </VStack>
        ))}
      </CardSection>

      <CardSection title={tr('Session')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(session.tone)}>{session.label}</Text>
        <Text size="footnote" tone="secondary">{session.headline}</Text>
        <Text size="footnote" tone="accent">{session.bar}</Text>
        <ValueRow label={tr('Session')}>{session.session}</ValueRow>
        <ValueRow label={tr('Unread')}>{session.unread}</ValueRow>
        {session.lines.map((item) => (
          <HStack key={item.id} gap={6}>
            <Text size="footnote" tone={toneOf(item.stateTone)}>{item.ok ? '●' : '○'}</Text>
            <Text size="footnote" tone={toneOf(item.stateTone)}>{`${item.label}: ${item.detail}`}</Text>
          </HStack>
        ))}
      </CardSection>

      <CardSection title={tr('Workspace')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(workspace.tone)}>{workspace.label}</Text>
        <Text size="footnote" tone="secondary">{workspace.headline}</Text>
        <ValueRow label={tr('Folder')}>{workspace.folder}</ValueRow>
        {workspace.lines.map((item) => (
          <HStack key={item.id} gap={6}>
            <Text size="footnote" tone={toneOf(item.stateTone)}>{item.set ? '●' : '○'}</Text>
            <Text size="footnote" tone={toneOf(item.stateTone)}>{`${item.label}: ${item.detail}`}</Text>
          </HStack>
        ))}
      </CardSection>

      <CardSection title={tr('Memory')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(memory.tone)}>{memory.label}</Text>
        <Text size="footnote" tone="secondary">{memory.headline}</Text>
        <ValueRow label={tr('Shared notes')}>{memory.shared}</ValueRow>
        <Text size="footnote" tone="tertiary">{memory.sharedDetail}</Text>
        {memory.lines.map((item) => (
          <HStack key={item.id + item.title} gap={6}>
            <Text size="footnote" tone={toneOf(item.stateTone)}>{item.on ? '●' : '○'}</Text>
            <Text size="footnote" tone={toneOf(item.stateTone)}>{`${item.title}: ${item.detail}`}</Text>
          </HStack>
        ))}
      </CardSection>

      <CardSection title={tr('Model tuning')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(tuning.tone)}>{tuning.effort}</Text>
        <Text size="footnote" tone="secondary">{tuning.headline}</Text>
        <ValueRow label={tr('Options')}>{tuning.count}</ValueRow>
        {tuning.enabled.length ? (
          <VStack gap={2}>
            {tuning.enabled.map((opt) => (
              <HStack key={opt.title} gap={6}>
                <Text size="footnote" tone="tertiary">{opt.title}</Text>
                <Text size="footnote" tone="accent">{opt.value}</Text>
              </HStack>
            ))}
          </VStack>
        ) : null}
        {tuning.options.length ? (
          <VStack gap={2}>
            {tuning.options.map((opt) => (
              <VStack key={opt.key} gap={0}>
                <Text size="footnote" tone={toneOf(opt.tone)}>{`${opt.title}: ${opt.value}`}</Text>
              </VStack>
            ))}
          </VStack>
        ) : null}
      </CardSection>

      <CardSection title={tr('Connected accounts')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(connectors.tone)}>{connectors.headline}</Text>
        {connectors.categories.length ? (
          <VStack gap={2}>
            {connectors.categories.map((cat) => (
              <HStack key={cat.kind} gap={6}>
                <Text size="footnote" tone={toneOf(cat.tone)}>{cat.kind}</Text>
                <Text size="footnote" tone="accent">{cat.bar}</Text>
                <Text size="footnote" tone="secondary">{String(cat.count)}</Text>
              </HStack>
            ))}
          </VStack>
        ) : null}
        {connectors.connectors.length ? (
          <VStack gap={2}>
            {connectors.connectors.map((item) => (
              <VStack key={item.name} gap={0}>
                <Text size="footnote" tone={toneOf(item.tone)}>{item.name}</Text>
                <Text size="footnote" tone="tertiary">{item.detail}</Text>
              </VStack>
            ))}
          </VStack>
        ) : null}
      </CardSection>

      <CardSection title={tr('Identity')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(identity.tone)}>{identity.headline}</Text>
        <ValueRow label={tr('Roster')}>{identity.band}</ValueRow>
        <ValueRow label={tr('Section')}>{identity.section}</ValueRow>
        <ValueRow label={tr('Rank')}>{String(identity.rank)}</ValueRow>
        <Text size="footnote" tone="tertiary">{`${identity.color} · ${identity.shape} · ${identity.look}`}</Text>
        {identity.traits.length ? (
          <VStack gap={2}>
            {identity.traits.map((trait) => (
              <HStack key={trait.id} gap={6}>
                <Text size="footnote" tone={toneOf(trait.stateTone)}>{trait.on ? '●' : '○'}</Text>
                <Text size="footnote" tone={toneOf(trait.stateTone)}>{`${trait.label}: ${trait.detail}`}</Text>
              </HStack>
            ))}
          </VStack>
        ) : null}
      </CardSection>

      <CardSection title={tr('Resilience')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(resilience.tone)}>{resilience.label}</Text>
        <Text size="footnote" tone="accent">{resilience.bar}</Text>
        <Text size="footnote" tone="secondary">{resilience.headline}</Text>
        {resilience.switches.length ? (
          <HStack gap={8}>
            {resilience.switches.map((s) => (
              <Text key={s.label} size="footnote" tone={s.on ? 'added' : 'tertiary'}>
                {`${s.on ? '●' : '○'} ${s.label}`}
              </Text>
            ))}
          </HStack>
        ) : null}
        {resilience.fallbacks.length ? (
          <VStack gap={2}>
            {resilience.fallbacks.map((f) => (
              <VStack key={f.backend + f.model} gap={0}>
                <Text size="footnote" tone={toneOf(f.tone)}>{f.label}</Text>
              </VStack>
            ))}
          </VStack>
        ) : null}
      </CardSection>

      <CardSection title={tr('Trust')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(trust.tone)}>{trust.label}</Text>
        <Text size="footnote" tone="secondary">{trust.headline}</Text>
        <ValueRow label={tr('Approval')}>{trust.modeLabel}</ValueRow>
        <Text size="footnote" tone="tertiary">{trust.modeDetail}</Text>
        {trust.rules.length ? (
          <VStack gap={2}>
            {trust.rules.map((rule) => (
              <VStack key={`${rule.pattern}-${rule.kind}`} gap={0}>
                <Text size="footnote" tone={toneOf(rule.tone)}>{`${rule.decision}: ${rule.pattern}`}</Text>
                {rule.kind ? <Text size="footnote" tone="tertiary">{rule.kind}</Text> : null}
              </VStack>
            ))}
          </VStack>
        ) : null}
      </CardSection>

      <CardSection title={tr('Skill library')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(library.tone)}>{library.headline}</Text>
        {library.kinds.length ? (
          <VStack gap={2}>
            {library.kinds.map((kind) => (
              <HStack key={kind.kind} gap={6}>
                <Text size="footnote" tone={toneOf(kind.tone)}>{kind.kind}</Text>
                <Text size="footnote" tone="accent">{kind.bar}</Text>
                <Text size="footnote" tone="secondary">{String(kind.count)}</Text>
              </HStack>
            ))}
          </VStack>
        ) : null}
        {library.skills.length ? (
          <VStack gap={2}>
            {library.skills.slice(0, 8).map((skill) => (
              <VStack key={skill.name} gap={0}>
                <Text size="footnote" tone={toneOf(skill.tone)}>{skill.name}</Text>
                <Text size="footnote" tone="tertiary">{skill.detail}</Text>
              </VStack>
            ))}
          </VStack>
        ) : null}
      </CardSection>

      <CardSection title={tr('Schedule')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(routineSummary.tone)}>{routineSummary.headline}</Text>
        <Text size="footnote" tone="accent">{routineSummary.bar}</Text>
        <ValueRow label={tr('Active')}>{String(routineSummary.active)}</ValueRow>
        <ValueRow label={tr('Paused')}>{String(routineSummary.paused)}</ValueRow>
        <ValueRow label={tr('Triggers')}>{routineSummary.mix}</ValueRow>
        {routineSummary.soonest.length ? (
          <VStack gap={2}>
            {routineSummary.soonest.map((item) => (
              <HStack key={item.id} gap={6}>
                <Text size="footnote" tone={toneOf(item.stateTone)}>{item.enabled ? '●' : '○'}</Text>
                <Text size="footnote" tone={toneOf(item.stateTone)}>{`${item.name} · ${item.trigger}`}</Text>
                {item.nextIn ? <Text size="footnote" tone="tertiary">{item.nextIn}</Text> : null}
              </HStack>
            ))}
          </VStack>
        ) : null}
      </CardSection>

      <CardSection title={tr('This week')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(week.tone)}>{week.verdict}</Text>
        <Text size="footnote" tone="secondary">{week.headline}</Text>
        <ValueRow label={tr('Steps')}>{week.totalLabel}</ValueRow>
        <ValueRow label={tr('Trend')}>{week.trend.label}</ValueRow>
        {week.peak ? (
          <Text size="footnote" tone="secondary">{tr('Busiest')}: {week.peak.label} · {week.peak.count}</Text>
        ) : null}
        {week.top.length ? (
          <VStack gap={2}>
            {week.top.map((day) => (
              <HStack key={day.label} gap={6}>
                <Text size="footnote" tone="tertiary">{day.label}</Text>
                <Text size="footnote" tone="accent">{day.bar}</Text>
                <Text size="footnote" tone="secondary">{String(day.count)}</Text>
              </HStack>
            ))}
          </VStack>
        ) : null}
      </CardSection>

      <CardSection title={tr('Capabilities')}>
        <Text size="footnote" tone="accent">{profile.bar}</Text>
        <Text size="subheadline" weight="semibold">{profile.headline}</Text>
        {profile.nextUp.length ? (
          <Text size="footnote" tone="secondary">
            {tr('Next up')}: {profile.nextUp.map((c) => c.title).join(' · ')}
          </Text>
        ) : null}
        {profile.caps.map((cap) => (
          <HStack key={cap.id} gap={6}>
            <Text size="footnote" tone={toneOf(cap.stateTone)}>{cap.enabled ? '●' : '○'}</Text>
            <Text size="footnote" tone={toneOf(cap.stateTone)}>{`${cap.title}: ${cap.detail}`}</Text>
          </HStack>
        ))}
      </CardSection>

      <CardSection title={tr('Health')}>
        <Text size="subheadline" weight="semibold" tone={toneOf(health.tone)}>{health.label}</Text>
        <Text size="footnote" tone="secondary">{health.headline}</Text>
        {health.checks.length ? (
          <VStack gap={2}>
            {health.checks.map((item) => (
              <VStack key={item.id} gap={0}>
                <Text size="footnote" tone={toneOf(item.tone)}>{`${item.title}: ${item.detail}`}</Text>
              </VStack>
            ))}
          </VStack>
        ) : null}
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
