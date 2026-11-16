import { useRef, useState, type ReactNode } from 'react'
import type { SkillUsage, SkillUsageRow, Usage, UsageProvider, UsageWindow } from '@shared/models'
import type { HostHealth } from '@shared/health'
import { healthLines } from '@shared/health'
import { IconButton, Reveal } from '../../components/Controls'
import { SlidingTabs } from '../../components/SlidingTabs'
import { HStack, Spacer, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { prefs, usePref } from '../../lib/prefs'
import { useResetLine, useTr } from '../../lib/use-language'
import { ProviderMascot } from '../usage/ProviderMascot'
import { UsageBar } from '../usage/UsageBar'
import { providerTintCSS, resetDescription, usageStatus, windowReading, windowTitle } from '../usage/usage-model'
import { UsageChooser } from './UsageChooser'
import './settings.css'

/** Every provider seen, in the order the host gives, so that one turned off can still fold away; and the ones the page opened with. */
function useProvidersSeen({ providers, services }: Usage) {
  const seen = useRef(new Map<string, UsageProvider>())
  const atFirst = useRef(new Set(providers.map((p) => p.id)))
  for (const provider of providers) seen.current.set(provider.id, provider)
  const rank = (id: string) => {
    // An isolated account (`agent:account`) sits right after its agent.
    const at = services?.findIndex((s) => s.id === id.split(':')[0]) ?? -1
    return at < 0 ? Number.MAX_SAFE_INTEGER : at
  }
  return { seen: [...seen.current.values()].sort((a, b) => rank(a.id) - rank(b.id)), atFirst: atFirst.current }
}

/** One computer's usage: its health, what its agents have used, and behind a button which agents show and in what order. */
export function UsageSection({ title, usage, skills, health, onShown, onOrder }: {
  title: ReactNode
  usage: Usage
  skills: SkillUsage | null
  health: HostHealth | null
  onShown: (id: string, shown: boolean) => void
  onOrder: (ids: string[]) => Promise<void>
}) {
  const tr = useTr()
  const [editing, setEditing] = useState(false)
  const [tab, setTab] = useState<'limits' | 'skills'>('limits')
  const canEdit = !!usage.services?.length && tab === 'limits'
  return (
    <VStack gap={10}>
      <HStack gap={6} className="settings-usage-computer">
        {title}
        <Spacer />
        {canEdit ? (
          <IconButton
            title={tr(editing ? 'Done' : 'Choose and arrange')}
            icon={editing ? 'checkmark' : 'slider.horizontal.3'}
            onClick={() => setEditing(!editing)}
          />
        ) : null}
      </HStack>
      {health ? <HealthStrip health={health} /> : null}
      <SlidingTabs<'limits' | 'skills'>
        items={[{ id: 'limits', label: tr('Limits') }, { id: 'skills', label: tr('Skills') }]}
        value={tab}
        onChange={(next) => { setTab(next); setEditing(false) }}
        ariaLabel={tr('Breakdown')}
      />
      {editing ? (
        <UsageChooser usage={usage} onShown={onShown} onOrder={onOrder} />
      ) : tab === 'limits' ? (
        <UsageLimits usage={usage} />
      ) : (
        <SkillBreakdown usage={skills} />
      )}
    </VStack>
  )
}

/** The computer's RAM, CPU and disk, read from `hostHealth`: an owner sees the machine their agents run on. */
function HealthStrip({ health }: { health: HostHealth }) {
  const tr = useTr()
  return (
    <div className="settings-usage-card settings-usage-health">
      {healthLines(health).map((line) => (
        <HStack key={line.label} gap={8} className="settings-usage-health-row">
          <Text size="caption" tone="secondary">{tr(line.label)}</Text>
          <Spacer />
          <Text size="caption" tone="text" tabular>{line.value}</Text>
        </HStack>
      ))}
    </div>
  )
}

/** How often each skill's text entered a turn, with its estimated tokens, cost and share. */
function SkillBreakdown({ usage }: { usage: SkillUsage | null }) {
  const tr = useTr()
  const rows = usage?.skills ?? []
  if (rows.length === 0) {
    return (
      <VStack gap={6} className="fade-in">
        <Text tone="secondary">{tr('No skill has been loaded yet.')}</Text>
        <Text size="caption" tone="secondary">{tr("A load is counted when an agent reads a skill's SKILL.md during a turn.")}</Text>
      </VStack>
    )
  }
  return (
    <div className="settings-usage-card settings-skills fade-in">
      <div className="settings-skill-row settings-skill-head">
        <Text size="caption" tone="secondary">{tr('Skill')}</Text>
        <Text size="caption" tone="secondary" tabular>{tr('Loads')}</Text>
        <Text size="caption" tone="secondary" tabular>{tr('Estimated tokens')}</Text>
        <Text size="caption" tone="secondary" tabular>{tr('Estimated cost')}</Text>
        <Text size="caption" tone="secondary" tabular>{tr('Share')}</Text>
        <Text size="caption" tone="secondary" tabular>{tr('Agents')}</Text>
      </div>
      {rows.map((row) => <SkillRow key={row.id} row={row} />)}
      <Text size="caption" tone="secondary" className="settings-skill-note">
        {tr("A load is counted when an agent reads a skill's SKILL.md during a turn.")}
      </Text>
    </div>
  )
}

function SkillRow({ row }: { row: SkillUsageRow }) {
  const tr = useTr()
  return (
    <div className="settings-skill-row">
      <Text tone="text" truncate>{row.name}</Text>
      <Text tone="text" tabular>{row.loads}</Text>
      <Text tone="text" tabular>{new Intl.NumberFormat().format(row.tokens)}</Text>
      <Text tone={row.costUsd === null ? 'secondary' : 'text'} tabular>{costText(row.costUsd, tr)}</Text>
      <Text tone="text" tabular>{Math.round(row.share * 100)}%</Text>
      <Text tone="text" tabular>{row.agents}</Text>
    </div>
  )
}

function costText(cost: number | null, tr: (key: string) => string): string {
  if (cost === null) return tr('Unavailable')
  if (cost === 0) return '$0.00'
  return cost < 0.01 ? tr('Under $0.01') : `$${cost.toFixed(2)}`
}

/** Every limit as a bar under its provider, the way the menu bar lists them. */
function UsageLimits({ usage }: { usage: Usage }) {
  const tr = useTr()
  const { seen, atFirst } = useProvidersSeen(usage)
  if (usage.providers.length === 0) return <Text tone="secondary">{tr('No usage information yet.')}</Text>
  return (
    <div className="settings-usage-card fade-in">
      {seen.map((provider) => (
        <Reveal key={provider.id} show={usage.providers.some((p) => p.id === provider.id)} animateOnMount={!atFirst.has(provider.id)}>
          <ProviderLimits provider={provider} />
        </Reveal>
      ))}
    </div>
  )
}

function ProviderLimits({ provider }: { provider: UsageProvider }) {
  const tr = useTr()
  const [style] = usePref(prefs.usageIconStyle)
  return (
    <VStack gap={12} className="settings-usage-provider">
      <HStack gap={8}>
        <ProviderMascot provider={provider} size={18} style={style} />
        <Text size="body" weight="semibold" tone="text">{provider.name}</Text>
      </HStack>
      {usageStatus(provider) ? <Text size="caption" tone="secondary">{tr(usageStatus(provider)!)}</Text> : null}
      {provider.windows.map((w) => <WindowLimit key={w.id} window={w} providerId={provider.id} />)}
    </VStack>
  )
}

/** A limit's name and when it resets, how much is used, and under them the bar; or the note an agent gives instead. */
function WindowLimit({ window: w, providerId }: { window: UsageWindow; providerId: string }) {
  const resetLine = useResetLine()
  const tr = useTr()
  return (
    <VStack gap={6} align="stretch">
      <HStack gap={8}>
        <Text tone="text" truncate>{tr(windowTitle(w))}</Text>
        {w.text ? (
          <Text tone="secondary" truncate grow className="settings-usage-note">{w.text}</Text>
        ) : (
          <>
            <Text tone="secondary" truncate grow>{resetLine(resetDescription(w) ?? '')}</Text>
            <Text tone="text" weight="medium" tabular>{windowReading(w, tr)}</Text>
          </>
        )}
      </HStack>
      {w.text ? null : <UsageBar percent={w.percent} tint={providerTintCSS(providerId)} />}
    </VStack>
  )
}
