import { useEffect, useState, type ReactNode } from 'react'
import { CardSection, IconButton } from '../../components/Controls'
import { Dialog } from '../../components/Overlay'
import { VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useStore } from '../../store/context'
import { errorText, type RoutineWebhook } from './routine'
import { LabeledRow } from './RoutineScheduleFields'
import './routines.css'
import { useTr } from '../../lib/use-language'


/** A saved webhook routine's address and key: copy either, reveal or replace the key. */
export function RoutineWebhookPanel({ botId, routineId }: { botId: string; routineId: string }) {
  const tr = useTr()
  const store = useStore()
  const [webhook, setWebhook] = useState<RoutineWebhook | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [showKey, setShowKey] = useState(false)
  const [copied, setCopied] = useState<'url' | 'key' | null>(null)
  const [confirmRotate, setConfirmRotate] = useState(false)
  const [rotating, setRotating] = useState(false)
  const [confirmGive, setConfirmGive] = useState(false)
  const [giving, setGiving] = useState(false)
  const [givenName, setGivenName] = useState<string | null>(null)
  const agentName = store.bots.get(botId)?.name ?? botId

  const load = async (rotate: boolean) => {
    setRotating(rotate)
    try {
      const value = await (await store.ready()).call<RoutineWebhook>('routineWebhook', { botId, id: routineId, rotate })
      setWebhook(value)
      if (rotate) {
        setCopied(null)
        setShowKey(true)
      }
      setError(null)
    } catch (e) {
      setError(errorText(e))
    } finally {
      setRotating(false)
    }
  }

  useEffect(() => {
    void load(false)
  }, [routineId])

  /** Rotates the key into the agent's own vault; the host returns only the saved key's name. */
  const give = async () => {
    const client = store.client
    if (!client) return
    setGiving(true)
    setError(null)
    try {
      const result = await client.call<{ name: string }>('routineCall', {
        botId,
        name: 'give_webhook_secret',
        arguments: { id: routineId },
      })
      setGivenName(result.name)
      setShowKey(false)
      await load(false)
    } catch (e) {
      setError(errorText(e))
    } finally {
      setGiving(false)
    }
  }

  const row = (label: string, text: string, copy: 'url' | 'key', extra?: ReactNode) => (
    <LabeledRow label={label} className="webhook-row">
      <Text size="callout" mono lines={2} grow className="routine-webhook-text">
        {text}
      </Text>
      {extra}
      <IconButton
        title={copied === copy ? 'Copied' : 'Copy'}
        icon={copied === copy ? 'checkmark' : 'doc.on.doc'}
        onClick={() => {
          if (!webhook) return
          window.codync.app.copy(copy === 'key' ? webhook.key : (webhook.url ?? webhook.localUrl))
          setCopied(copy)
        }}
      />
    </LabeledRow>
  )

  return (
    <VStack gap={10}>
      {webhook ? (
        <CardSection
          footer={
            webhook.url
              ? 'POST with Authorization: Bearer <key>. For GitHub, use content type application/json and the key as the secret. ' +
                tr('Deliveries wait up to 72 hours while this computer is off. They pass through the Codync cloud, which can read them.')
              : webhook.needsAccount
                ? "This computer isn't in an account, so only it can send to this address. Add it to your account for a public one."
                : 'The Codync cloud is off, so only this computer can send to it.'
          }
        >
          {row(webhook.url ? tr('URL') : tr('Local URL'), webhook.url ?? webhook.localUrl, 'url')}
          {row(
            'Key',
            showKey ? webhook.key : '•'.repeat(16),
            'key',
            <>
              <IconButton title={tr(showKey ? 'Hide key' : 'Show key')} icon={showKey ? 'eye.slash' : 'eye'} onClick={() => setShowKey((s) => !s)} />
              <IconButton title="Replace key" icon="arrow.triangle.2.circlepath" disabled={rotating || store.isOffline} onClick={() => setConfirmRotate(true)} />
              <IconButton
                title={`Give the signing secret to ${agentName}`}
                icon="key"
                disabled={giving || rotating || store.isOffline}
                onClick={() => setConfirmGive(true)}
              />
            </>,
          )}
          {givenName ? <Text size="caption" tone="secondary">{`Saved as ${givenName}`}</Text> : null}
        </CardSection>
      ) : error ? (
        <Text size="caption" tone="danger" className="routine-caption">{error}</Text>
      ) : (
        <Text size="caption" tone="secondary" className="routine-caption">{tr(`Loading webhook…`)}</Text>
      )}
      <Dialog
        open={confirmRotate}
        title={tr('Replace the key?')}
        message={tr('Senders using the current key stop working until they get the new one.')}
        actions={[{ title: 'Replace key', destructive: true, action: () => void load(true) }]}
        onClose={() => setConfirmRotate(false)}
      />
      <Dialog
        open={confirmGive}
        title={`Give the signing secret to ${agentName}?`}
        message={tr('The old secret stops working. The agent gets the new one in its own vault; the value is never shown here.')}
        actions={[{ title: 'Give secret', destructive: true, action: () => void give() }]}
        onClose={() => setConfirmGive(false)}
      />
    </VStack>
  )
}
