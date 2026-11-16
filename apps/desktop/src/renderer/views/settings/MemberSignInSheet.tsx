import { useEffect, useState } from 'react'
import type { MemberProfile, MemberSignedIn } from '@shared/models'
import { Button } from '../../components/Controls'
import { ModalHeader, Sheet } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { LoopbackTransport } from '../../client/host-client'
import { useTr } from '../../lib/use-language'
import { useApp } from '../../store/context'
import { Field } from '../marketplace/pieces'

/** Where this computer's member session is kept between runs. A token is per computer. */
const STORAGE = 'codyncMemberSession'

/** The platform label the host records for this session device. */
const PLATFORM =
  window.codync.platform === 'darwin' ? 'macos' : window.codync.platform === 'win32' ? 'windows' : 'linux'

interface Saved {
  token: string
  expiresAt: number
}

function saved(): Saved | null {
  try {
    const raw = localStorage.getItem(STORAGE)
    if (!raw) return null
    const value = JSON.parse(raw) as Saved
    if (!value.token || (value.expiresAt && value.expiresAt <= Date.now())) return null
    return value
  } catch {
    return null
  }
}

/**
 * Signing in as a member on this computer (docs/features/people.md): redeem the invitation code the
 * owner gave you, or the company email your directory invited. The host answers with a member
 * session token, and every call after that acts as that person: their bots, chats and connectors,
 * and nothing of the owner's. The token is a session on the computer's own API, so ending it in
 * People revokes it here too.
 */
export function MemberSignInSheet({ open, onClose }: { open: boolean; onClose: () => void }) {
  const tr = useTr()
  const app = useApp()
  const [code, setCode] = useState('')
  const [email, setEmail] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [session, setSession] = useState<Saved | null>(saved)
  const [profile, setProfile] = useState<MemberProfile | null>(null)
  const [computer, setComputer] = useState('')
  const [connectors, setConnectors] = useState<string[]>([])

  const owner = () => {
    const local = app.host.local
    return local ? { baseURL: local.baseURL, token: local.token } : null
  }

  /** What the member's own token sees: who they are, the computer, and their connector list. */
  const openSession = async (value: Saved) => {
    const local = owner()
    if (!local) return
    const transport = new LoopbackTransport(local.baseURL, value.token)
    const hello = await transport.call('hello', {}, 20_000) as { name?: string; member?: MemberProfile }
    const listed = await transport.call('connectors', {}, 20_000).catch(() => null) as
      | { items?: { name?: string }[] }
      | null
    setProfile(hello.member ?? null)
    setComputer(hello.name ?? '')
    setConnectors((listed?.items ?? []).map((item) => item.name ?? '').filter(Boolean))
    setSession(value)
  }

  useEffect(() => {
    if (!open) return
    const value = saved()
    if (!value) {
      setSession(null)
      setProfile(null)
      return
    }
    void openSession(value).catch(() => {
      localStorage.removeItem(STORAGE)
      setSession(null)
      setProfile(null)
    })
  }, [open, app])

  const start = async (found: MemberSignedIn) => {
    const value = { token: found.token, expiresAt: found.expiresAt }
    localStorage.setItem(STORAGE, JSON.stringify(value))
    await openSession(value)
    setCode('')
    setEmail('')
  }
  const accept = async () => {
    const local = owner()
    if (!local) return
    setBusy(true)
    try {
      const answer = await new LoopbackTransport(local.baseURL, local.token).call(
        'acceptMemberInvite',
        { code: code.trim(), name: tr('Codync desktop'), platform: PLATFORM },
        20_000,
      ) as MemberSignedIn
      await start(answer)
      setError(null)
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason))
    }
    setBusy(false)
  }
  const signIn = async () => {
    const local = owner()
    if (!local) return
    setBusy(true)
    try {
      const answer = await new LoopbackTransport(local.baseURL, local.token).call(
        'memberSignIn',
        { email: email.trim(), name: tr('Codync desktop'), platform: PLATFORM },
        20_000,
      ) as MemberSignedIn
      await start(answer)
      setError(null)
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason))
    }
    setBusy(false)
  }
  const signOut = () => {
    localStorage.removeItem(STORAGE)
    setSession(null)
    setProfile(null)
    setConnectors([])
    setComputer('')
  }

  const name = profile?.person?.name ?? tr('Member')
  return (
    <Sheet open={open} onClose={onClose} width={380}>
      <ModalHeader title={tr('Sign in as a member')} onClose={onClose} />
      {session && profile ? (
        <VStack gap={10} className="settings-stack">
          <VStack gap={4} className="settings-stack">
            <Text weight="semibold">{name}</Text>
            <Text size="caption" tone="secondary">
              {profile.role === 'admin' ? tr('Admin on this computer') : tr('Member on this computer')}
              {computer ? ` · ${computer}` : ''}
            </Text>
            {profile.person?.email ? <Text size="caption" tone="tertiary">{profile.person.email}</Text> : null}
          </VStack>
          <VStack gap={4} className="settings-stack">
            <Text size="caption" tone="secondary">{tr('Connectors you share or own')}</Text>
            {connectors.length === 0 ? (
              <Text size="caption" tone="tertiary">{tr('None yet')}</Text>
            ) : (
              connectors.map((connector) => <Text key={connector} size="caption">{connector}</Text>)
            )}
          </VStack>
          <Text size="caption" tone="tertiary">
            {tr('Your bots, chats and connectors open in Codync as you. The owner can end this session in People.')}
          </Text>
          <HStack gap={6}>
            <Button kind="secondary" onClick={signOut}>{tr('Sign out')}</Button>
          </HStack>
        </VStack>
      ) : (
        <VStack gap={10} className="settings-stack">
          <Text size="caption" tone="secondary">
            {tr('Paste the invitation code the owner gave you. It works once, and this app then acts as you.')}
          </Text>
          <Field label={tr('Invitation code')} value={code} onChange={setCode} placeholder={tr('Paste your code')} />
          <HStack gap={6}>
            <Button disabled={busy || !code.trim()} onClick={() => void accept()}>
              {busy ? tr('Signing in…') : tr('Sign in')}
            </Button>
          </HStack>
          <Text size="caption" tone="secondary">
            {tr('Or sign in with your company email, when the owner set up company sign-in.')}
          </Text>
          <Field label={tr('Company email')} value={email} onChange={setEmail} placeholder="person@example.com" />
          <HStack gap={6}>
            <Button kind="secondary" disabled={busy || !email.trim()} onClick={() => void signIn()}>
              {tr('Continue with email')}
            </Button>
          </HStack>
          {error ? <Text role="alert" tone="danger">{error}</Text> : null}
        </VStack>
      )}
    </Sheet>
  )
}
