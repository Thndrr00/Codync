import { useState } from 'react'
import type { Appearance } from '@shared/appearance'
import { Button, CardSection, Reveal, Switch } from '../../components/Controls'
import { Dialog } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { normalizeRosterViewFilter } from '../../lib/roster-view-filter'
import { prefs } from '../../lib/prefs'
import { exportSettings, parseSettings, SETTINGS_GROUPS, type ParseResult, type SettingsGroup, type SettingsSource, type Values } from '../../lib/settings-io'
import type { YourBubbles } from '../../lib/theme'
import { useTr } from '../../lib/use-language'
import { inputMode, talkKey, type TalkKey, type VoiceInputMode } from './voice-settings'

const allGroups = () => Object.fromEntries(SETTINGS_GROUPS.map((g) => [g.id, true])) as Record<SettingsGroup, boolean>

/** The device-local prefs that travel in a settings file; never a key, token, account or host value. */
function snapshot(appearance: Appearance): SettingsSource {
  return {
    theme: { appearance },
    roster: { rosterViewFilter: prefs.rosterViewFilter.get() },
    bubbles: { yourBubbles: prefs.yourBubbles.get() },
    motion: { movingAgentFaces: prefs.movingAgentFaces.get(), backgroundBlur: prefs.backgroundBlur.get() },
    voice: { inputMode: inputMode.get(), talkKey: talkKey.get() },
    appearance: { accentColor: prefs.accentColor.get() },
  }
}

function show(value: unknown, tr: (s: string) => string): string {
  if (value === null || value === undefined) return tr('Default')
  if (typeof value === 'boolean') return value ? tr('On') : tr('Off')
  if (typeof value === 'object') return JSON.stringify(value)
  return String(value)
}

/** One line under a group's name in the import preview: its fields and the values about to be set. */
function describe(values: Values, tr: (s: string) => string): string {
  return Object.entries(values).map(([key, value]) => `${key}: ${show(value, tr)}`).join(', ')
}

interface Pending {
  result: Extract<ParseResult, { ok: true }>
  chosen: Set<SettingsGroup>
}

/** Settings › General: move this device's UI preferences to a JSON file and back. */
export function SettingsIOSection() {
  const tr = useTr()
  const [included, setIncluded] = useState<Record<SettingsGroup, boolean>>(allGroups)
  const [pending, setPending] = useState<Pending | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)

  const exportNow = async () => {
    try {
      const appearance = await window.codync.app.appearance()
      const json = exportSettings(snapshot(appearance), SETTINGS_GROUPS.map((g) => g.id).filter((id) => included[id]))
      const blob = new Blob([json], { type: 'application/json' })
      const url = URL.createObjectURL(blob)
      const link = document.createElement('a')
      link.href = url
      link.download = `codync-settings-${new Date().toISOString().slice(0, 10)}.json`
      document.body.appendChild(link)
      link.click()
      link.remove()
      setTimeout(() => URL.revokeObjectURL(url), 0)
      setError(null)
      setNotice(tr('Settings exported.'))
    } catch (e) {
      setNotice(null)
      setError(e instanceof Error ? e.message : String(e))
    }
  }

  const importNow = async () => {
    try {
      const files = await window.codync.app.pickFiles()
      const file = files[0]
      if (!file) return
      const result = parseSettings(new TextDecoder().decode(file.data))
      if (!result.ok) {
        setNotice(null)
        setError(tr(result.error))
        return
      }
      setError(null)
      setNotice(null)
      setPending({ result, chosen: new Set(Object.keys(result.groups) as SettingsGroup[]) })
    } catch (e) {
      setNotice(null)
      setError(e instanceof Error ? e.message : String(e))
    }
  }

  const confirm = () => {
    if (!pending) return
    const { groups } = pending.result
    const picked = pending.chosen
    if (picked.has('theme') && groups.theme) void window.codync.app.setAppearance(groups.theme.appearance as Appearance)
    if (picked.has('roster') && groups.roster) prefs.rosterViewFilter.set(normalizeRosterViewFilter(groups.roster.rosterViewFilter))
    if (picked.has('bubbles') && groups.bubbles) prefs.yourBubbles.set(groups.bubbles.yourBubbles as YourBubbles)
    if (picked.has('motion') && groups.motion) {
      if (typeof groups.motion.movingAgentFaces === 'boolean') prefs.movingAgentFaces.set(groups.motion.movingAgentFaces)
      if (typeof groups.motion.backgroundBlur === 'boolean') prefs.backgroundBlur.set(groups.motion.backgroundBlur)
    }
    if (picked.has('voice') && groups.voice) {
      if (typeof groups.voice.inputMode === 'string') inputMode.set(groups.voice.inputMode as VoiceInputMode)
      if (groups.voice.talkKey) talkKey.set(groups.voice.talkKey as TalkKey)
    }
    if (picked.has('appearance') && groups.appearance) prefs.accentColor.set(groups.appearance.accentColor as string | null)
    setPending(null)
    setNotice(tr('Settings imported.'))
  }

  return (
    <>
      <CardSection title="Settings file" footer={tr('Only this device’s appearance and UI preferences. No keys, accounts or computer data are included.')}>
        <Text size="callout" tone="secondary">{tr('Include in an export')}</Text>
        {SETTINGS_GROUPS.map((group) => (
          <Switch key={group.id} on={included[group.id]} onChange={(on) => setIncluded((prev) => ({ ...prev, [group.id]: on }))}>
            {tr(group.label)}
          </Switch>
        ))}
        <HStack gap={8}>
          <Button kind="secondary" onClick={() => void exportNow()}>{tr('Export settings')}</Button>
          <Button kind="secondary" onClick={() => void importNow()}>{tr('Import settings')}</Button>
        </HStack>
        {error ? <Text size="caption" tone="danger" role="alert">{error}</Text> : null}
        <Reveal show={!!notice}>
          {notice ? <Text size="caption" tone="secondary">{notice}</Text> : null}
        </Reveal>
      </CardSection>
      <Dialog
        open={pending !== null}
        title={tr('Import settings')}
        message={pending ? tr('Choose the groups to apply. The rest of this device’s settings stay as they are.') : null}
        actions={[{ title: tr('Import selected'), disabled: !pending || pending.chosen.size === 0, action: confirm }]}
        onClose={() => setPending(null)}
      >
        {pending ? (
          <VStack gap={6}>
            {SETTINGS_GROUPS.filter((g) => g.id in pending.result.groups).map((group) => {
              const values = pending.result.groups[group.id]!
              return (
                <VStack key={group.id} gap={2}>
                  <Switch on={pending.chosen.has(group.id)} onChange={(on) => setPending((prev) => {
                    if (!prev) return prev
                    const chosen = new Set(prev.chosen)
                    if (on) chosen.add(group.id)
                    else chosen.delete(group.id)
                    return { ...prev, chosen }
                  })}>
                    {tr(group.label)}
                  </Switch>
                  <Text size="caption2" tone="tertiary">{describe(values, tr)}</Text>
                </VStack>
              )
            })}
            {pending.result.skipped.length ? (
              <Text size="caption" tone="attention">
                {tr('Skipped: {items}').replace('{items}', pending.result.skipped.map((s) => s.reason).join(' '))}
              </Text>
            ) : null}
          </VStack>
        ) : null}
      </Dialog>
    </>
  )
}
