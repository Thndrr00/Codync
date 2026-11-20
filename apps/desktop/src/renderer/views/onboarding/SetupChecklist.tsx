import { useEffect, useRef, useState } from 'react'
import { IconButton } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { HStack, Spacer, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useModels, useModel } from '../../lib/observable'
import { prefs, usePref } from '../../lib/prefs'
import { useReduceMotion } from '../../lib/theme'
import { useTr } from '../../lib/use-language'
import { account } from '../../store/account'
import { useApp } from '../../store/context'
import { setupProgress, type SetupStepId } from './setup-checklist'
import './setup-checklist.css'

/**
 * A small, optional setup checklist beside the guided steps. Every row is derived from the live
 * stores (or a device pref for computer access and account), so it ticks over on its own.
 * Nothing here blocks the sheet: each row can be dismissed, and that choice is saved on this
 * device so it stays dismissed after a restart.
 */
export function SetupChecklist({ onExtensions, onAccess, onAccount }: { onExtensions?: () => void; onAccess?: () => void; onAccount?: () => void }) {
  const tr = useTr()
  const app = useApp()
  useModel(app)
  useModels([...app.stores.values()])
  useModel(account)
  const [accessSetup] = usePref(prefs.computerAccessSetup)
  const [dismissed, setDismissed] = usePref(prefs.onboardingChecklistDismissed)
  const reduceMotion = useReduceMotion()
  const [celebrating, setCelebrating] = useState(false)

  const local = useModel(app.local)
  const messageSent = [...app.stores.values()].some((store) =>
    [...store.bots.keys()].some((botId) => store.allEntries(botId).some((entry) => entry.kind === 'user' && !entry.data.internal)),
  )
  const completed: Record<SetupStepId, boolean> = {
    host: local !== null,
    access: accessSetup === 'ready',
    bot: (local?.roster.length ?? 0) > 0,
    message: messageSent,
    account: account.isSignedIn,
  }
  const progress = setupProgress(completed, dismissed)
  const wasComplete = useRef(progress.complete)
  useEffect(() => {
    const justCompleted = progress.complete && !wasComplete.current && !reduceMotion
    wasComplete.current = progress.complete
    if (!justCompleted) return
    setCelebrating(true)
    const timer = setTimeout(() => setCelebrating(false), 1200)
    return () => clearTimeout(timer)
  }, [progress.complete, reduceMotion])

  const titles: Record<SetupStepId, string> = {
    host: tr('This computer’s host'),
    access: tr('Computer access'),
    bot: tr('A bot'),
    message: tr('First message'),
    account: tr('Codync account'),
  }
  const notes: Record<SetupStepId, string> = {
    host: completed.host ? tr('Answering') : tr('Not running'),
    access: completed.access ? tr('Granted') : tr('Grant it to use apps and the screen'),
    bot: completed.bot ? tr('Ready') : tr('Create one to get started'),
    message: completed.message ? tr('Sent') : tr('Say hello to a bot'),
    account: completed.account ? tr('Signed in') : tr('Optional · sync across devices'),
  }
  const actionFor = (id: SetupStepId): (() => void) | undefined => {
    if (id === 'access') return onAccess
    if (id === 'bot' || id === 'message') return onExtensions
    if (id === 'account') return onAccount
    return undefined
  }

  return (
    <VStack gap={8} className="setup-checklist">
      <HStack gap={8}>
        <Text size="headline" weight="semibold">{tr('Set up Codync')}</Text>
        <Spacer />
        <Text size="caption" tone="secondary" tabular>{tr('{done} of {total}').replace('{done}', String(progress.done)).replace('{total}', String(progress.total))}</Text>
      </HStack>
      <div className="setup-progress" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(progress.fraction * 100)} aria-label={tr('Setup progress')}>
        <div className="setup-progress-fill" style={{ width: `${progress.fraction * 100}%` }} />
      </div>
      <VStack gap={2}>
        {progress.visible.map((id) => {
          const action = actionFor(id)
          return (
            <HStack key={id} gap={8} className={`setup-step${completed[id] ? ' done' : ''}`}>
              <span className="setup-step-mark" aria-hidden>
                {completed[id] ? <Icon name="checkmark.circle.fill" size={15} /> : <Icon name="circle" size={15} />}
              </span>
              <VStack gap={0} grow>
                <Text size="callout" weight="medium" tone={completed[id] ? 'secondary' : 'text'}>{titles[id]}</Text>
                <Text size="caption" tone="tertiary">{notes[id]}</Text>
              </VStack>
              {!completed[id] && action ? <button type="button" className="setup-step-open" onClick={action}>{tr('Open')}</button> : null}
              <IconButton title={tr('Dismiss this step')} icon="xmark" size={22} onClick={() => { if (!dismissed.includes(id)) setDismissed([...dismissed, id]) }} />
            </HStack>
          )
        })}
      </VStack>
      {progress.complete ? (
        <HStack gap={6} className={`setup-complete${celebrating ? ' celebrate' : ''}`}>
          <Icon name="checkmark.circle.fill" size={15} />
          <Text size="caption" tone="secondary">{tr('All set — you’re ready to go.')}</Text>
        </HStack>
      ) : null}
      {dismissed.length > 0 ? <button type="button" className="setup-restore" onClick={() => setDismissed([])}>{tr('Show dismissed steps')}</button> : null}
    </VStack>
  )
}
