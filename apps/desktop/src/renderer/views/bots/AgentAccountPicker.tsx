import { useEffect, useRef, useState } from 'react'
import type { AgentAccount } from '@shared/models'
import { Button, ChoicePicker, IconButton, Spinner } from '../../components/Controls'
import { HStack, Spacer, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'

interface Props {
  backend: string
  selection?: string | null
  online: boolean
  contextKey: string
  load: () => Promise<AgentAccount[]>
  authenticate: () => Promise<AgentAccount>
  onChange: (id: string | null) => void
}

/** Account lists belong to one computer; a context change discards their pending results. */
export function AgentAccountPicker(props: Props) {
  if (props.backend !== 'antigravity') return null
  return <AccountPicker key={JSON.stringify([props.contextKey, props.backend, props.online])} {...props} />
}

function AccountPicker({ selection, online, load, authenticate, onChange }: Props) {
  const [accounts, setAccounts] = useState<AgentAccount[]>([])
  const [loading, setLoading] = useState(online)
  const [signingIn, setSigningIn] = useState(false)
  const [failure, setFailure] = useState<string | null>(null)
  const [retry, setRetry] = useState(0)
  const alive = useRef(false)
  const callbacks = useRef({ load, authenticate, onChange })
  callbacks.current = { load, authenticate, onChange }

  useEffect(() => {
    alive.current = true
    return () => { alive.current = false }
  }, [])

  useEffect(() => {
    let cancelled = false
    if (!online) return
    setLoading(true)
    setFailure(null)
    void callbacks.current.load().then((value) => {
      if (!cancelled) setAccounts(value.filter((account) => account.backend === 'antigravity'))
    }).catch((error: unknown) => {
      if (!cancelled) setFailure(`Couldn't load accounts: ${message(error)}`)
    }).finally(() => {
      if (!cancelled) setLoading(false)
    })
    return () => { cancelled = true }
  }, [online, retry])

  const signIn = async () => {
    setSigningIn(true)
    setFailure(null)
    try {
      const account = await callbacks.current.authenticate()
      if (!alive.current) return
      setAccounts((current) => [...current.filter((item) => item.id !== account.id), account])
      callbacks.current.onChange(account.id)
    } catch (error) {
      if (alive.current) setFailure(`Couldn't sign in: ${message(error)}`)
    } finally {
      if (alive.current) setSigningIn(false)
    }
  }
  const options = [{ id: '', label: 'Default account' }, ...accounts.map((account) => ({ id: account.id, label: account.name }))]
  if (selection && !accounts.some((account) => account.id === selection)) {
    options.push({ id: selection, label: 'Saved account (not loaded)' })
  }

  return (
    <VStack gap={6}>
      <HStack gap={8}>
        <Text tone="text">Account</Text>
        <Spacer />
        {loading || signingIn ? <Spinner size={12} /> : (
          <IconButton title={failure ? 'Retry account list' : 'Refresh account list'} icon="arrow.clockwise" disabled={!online} onClick={() => setRetry((value) => value + 1)} />
        )}
        <ChoicePicker selection={selection ?? ''} options={options} onChange={(id) => onChange(id || null)} />
      </HStack>
      <Button kind="secondary" disabled={!online || loading || signingIn} onClick={() => { void signIn() }}>
        {signingIn ? 'Signing in…' : 'Sign in to another account'}
      </Button>
      <div role="status" aria-live="polite">
        {failure ? <Text size="caption" tone="attention">{failure} Retry or sign in again.</Text> : !online ? (
          <Text size="caption" tone="secondary">Connect to this computer to load accounts or sign in.</Text>
        ) : !loading && accounts.length === 0 ? (
          <Text size="caption" tone="secondary">No isolated accounts. Use the default account or sign in to add one.</Text>
        ) : null}
      </div>
      <Text size="caption" tone="secondary">Each account keeps its own sign-in on this computer. Changing accounts starts a new agent session; chat history is kept.</Text>
    </VStack>
  )
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : 'Try again.'
}
