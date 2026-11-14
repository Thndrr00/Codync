import { useEffect, useState } from 'react'
import type { HostClient } from '../../client/host-client'
import { Button, CardSection, Spinner } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { HStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'
import { errorText, liveClient } from './parts'
import * as voice from './voice-settings'

/**
 * Live voice with the computer's own Codex / ChatGPT sign-in (OpenBot 0.65). The sign-in lives in
 * Codex; Codync reads whether it can carry a realtime conversation and says why not when it can't.
 * A ChatGPT sign-in brokers the call's WebRTC offer through Codex, so no OpenAI key is needed.
 */
export function CodexVoiceSettings() {
  const tr = useTr()
  const store = useStore()
  const [access, setAccess] = useState<voice.CodexVoiceAccess | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const check = (client: HostClient) => {
    setBusy(true)
    setError(null)
    void voice
      .voiceStatus(client)
      .then((status) => setAccess(status.codex ?? { available: false, reason: null, account: null }))
      .catch((e: unknown) => setError(errorText(e)))
      .finally(() => setBusy(false))
  }

  useEffect(() => {
    const client = liveClient(store)
    if (client) check(client)
  }, [])

  const detail = access?.available
    ? access.account
      ? tr('Signed in with ChatGPT as {account}.').replaceAll('{account}', () => access.account ?? '')
      : tr('Signed in with ChatGPT in Codex.')
    : (access?.reason ?? tr('Codync is checking the sign-in.'))

  return (
    <CardSection
      title={tr('Use my Codex sign-in')}
      footer={tr('Live voice through Codex never asks for an OpenAI key: the call runs on your ChatGPT plan.')}
    >
      <HStack gap={8} className="settings-row">
        {busy ? (
          <Spinner />
        ) : (
          <Icon name={access?.available ? 'checkmark.circle.fill' : 'info.circle'} size={12} color={access?.available ? 'var(--accent)' : 'var(--tertiary)'} />
        )}
        <Text size="footnote" tone={access?.available ? 'secondary' : 'tertiary'} grow>
          {error ?? (access ? detail : tr('Codync is checking the sign-in.'))}
        </Text>
        <Button kind="secondary" onClick={() => {
          const client = liveClient(store)
          if (client) check(client)
        }} disabled={busy || !liveClient(store)}>
          {tr('Check again')}
        </Button>
      </HStack>
    </CardSection>
  )
}
