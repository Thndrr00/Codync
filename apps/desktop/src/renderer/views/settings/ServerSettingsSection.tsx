import { useEffect, useState } from 'react'
import { Button, CardSection, ChoicePicker, ValueRow } from '../../components/Controls'
import { Dialog } from '../../components/Overlay'
import { useModel } from '../../lib/observable'
import { useTr } from '../../lib/use-language'
import { useApp } from '../../store/context'
import type { ServerSettings } from '../../store/bot-store'
import { SwitchRow } from '../bots/parts'
import { errorText } from './parts'

/**
 * Settings that belong to this computer rather than to one bot: how its agents post updates between
 * steps ("Personality"), whether Claude agents load the person's own Claude Code setup, and who may
 * control this computer over remote desktop. Ported from OpenBot's server personality, the Claude Code
 * setup switch and the remote desktop policy.
 */
export function ServerSettingsSection() {
  const app = useApp()
  const local = useModel(app.local)
  const tr = useTr()
  const [error, setError] = useState<string | null>(null)
  const [closed, setClosed] = useState<number | null>(null)
  const [confirmingStop, setConfirmingStop] = useState(false)
  useEffect(() => {
    void local?.refreshServerSettings()
  }, [local])
  const settings = local?.serverSettings
  if (!settings) return null
  const save = async (patch: Partial<ServerSettings>) => {
    setError(null)
    try {
      await local?.setServerSettings(patch)
    } catch (e) {
      setError(errorText(e))
    }
  }
  const stopAll = async () => {
    setError(null)
    try {
      setClosed((await local?.stopAllRemoteControl()) ?? 0)
    } catch (e) {
      setError(errorText(e))
    }
  }
  return (
    <>
      <CardSection
        title={tr('Personality')}
        footer={tr(
          'How the agents on this computer post updates between their steps. Natural writes a short note only when a finding changes the plan or before a long step, then an answer that starts with the verdict. Classic shows their own text as it comes.',
        )}
      >
        <ValueRow label={tr('Updates between steps')}>
          <ChoicePicker
            selection={settings.responseStyle}
            options={[
              { id: 'natural', label: tr('Natural') },
              { id: 'classic', label: tr('Classic') },
            ]}
            onChange={(responseStyle) => void save({ responseStyle: responseStyle as ServerSettings['responseStyle'] })}
          />
        </ValueRow>
      </CardSection>

      <CardSection
        title={tr('Claude Code')}
        footer={tr(
          "Off by default: your agents do not load your CLAUDE.md, skills, agents, hooks or plugins from ~/.claude, so each turn sends a shorter prompt. Your sign-in settings in ~/.claude/settings.json still apply, and the files of an agent's workspace still load.",
        )}
      >
        <SwitchRow
          title={tr('Use my Claude Code setup')}
          detail={tr('Give Claude agents your own Claude Code configuration. Turn this on if they should use your CLAUDE.md, skills or hooks.')}
          on={settings.claudeUserSetup}
          onChange={(claudeUserSetup) => void save({ claudeUserSetup })}
        />
      </CardSection>

      <CardSection
        title={tr('Remote desktop')}
        footer={tr(
          'Every device you paired may control this computer. Codync has one owner and no member roles yet, so "Owners and admins" reads as every device; "Only this computer" never lets a phone control it.',
        )}
      >
        <ValueRow label={tr('Who can control')}>
          <ChoicePicker
            selection={settings.remoteControl.access}
            options={[
              { id: 'everyone', label: tr('Every device') },
              { id: 'admins', label: tr('Owners and admins') },
              { id: 'owner', label: tr('Only this computer') },
            ]}
            onChange={(access) =>
              void save({
                remoteControl: { ...settings.remoteControl, access: access as ServerSettings['remoteControl']['access'] },
              })
            }
          />
        </ValueRow>
        <SwitchRow
          title={tr('Ask on this computer')}
          detail={tr('Allow or deny each session here. A session nobody allows in 30 seconds does not start.')}
          on={settings.remoteControl.askOnHost}
          onChange={(askOnHost) => void save({ remoteControl: { ...settings.remoteControl, askOnHost } })}
        />
        <ValueRow
          label={tr('Sessions')}
          detail={
            closed === null
              ? tr('Ends every remote control session now, without turning Remote screen off.')
              : tr('Stopped {count} session(s).').replace('{count}', String(closed))
          }
        >
          <Button kind="secondary" onClick={() => setConfirmingStop(true)}>
            {tr('Stop all')}
          </Button>
        </ValueRow>
      </CardSection>

      <Dialog
        open={confirmingStop}
        title={tr('Stop all remote control?')}
        message={tr('Ends every remote control session now, without turning Remote screen off.')}
        cancel="Cancel"
        onClose={() => setConfirmingStop(false)}
        actions={[{ title: 'Stop all', destructive: true, action: () => void stopAll() }]}
      />

      {error ? (
        <CardSection title={tr('Something went wrong')}>
          <ValueRow label={tr('Error')}>{error}</ValueRow>
        </CardSection>
      ) : null}
    </>
  )
}
