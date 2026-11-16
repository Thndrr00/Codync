import { useEffect, useMemo, useState } from 'react'
import { Button, CardSection, ChoicePicker } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useModel } from '../../lib/observable'
import { useApp } from '../../store/context'
import { Field } from '../marketplace/pieces'

interface Account {
  id: string
  name: string
  address: string
  imapHost: string
  smtpHost: string
  configured: boolean
}

interface Message {
  uid: string
  from: string
  subject: string
  date: string
}

/** The email accounts this computer sends and reads with, and their newest messages. */
export function EmailSection() {
  const app = useApp()
  const local = useModel(app.local)
  const running = local !== null && local.connection.kind === 'online'
  const [accounts, setAccounts] = useState<Account[]>([])
  const [account, setAccount] = useState('')
  const [messages, setMessages] = useState<Message[]>([])
  const [name, setName] = useState('')
  const [address, setAddress] = useState('')
  const [password, setPassword] = useState('')
  const [imapHost, setImapHost] = useState('')
  const [smtpHost, setSmtpHost] = useState('')
  const [username, setUsername] = useState('')
  const [to, setTo] = useState('')
  const [subject, setSubject] = useState('')
  const [body, setBody] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const load = useMemo(() => async () => {
    if (!local) return
    try {
      const client = await local.ready()
      const list = await client.call<{ accounts: Account[] }>('emailAccounts', {})
      setAccounts(list.accounts)
      setAccount((current) => current || list.accounts[0]?.id || '')
      setError(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
  }, [local])

  useEffect(() => {
    if (running) void load()
  }, [running, load])

  const readInbox = async () => {
    if (!local || !account) return
    setBusy(true)
    try {
      const client = await local.ready()
      const answer = await client.call<{ messages: Message[] }>('emailInbox', { account, limit: 20 }, 60_000)
      setMessages(answer.messages)
      setError(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
    setBusy(false)
  }

  const addAccount = async () => {
    if (!local || !address.trim() || !imapHost.trim() || !smtpHost.trim()) return
    setBusy(true)
    try {
      const client = await local.ready()
      await client.call('saveEmailAccount', {
        name: name.trim() || address.trim(),
        address: address.trim(),
        password,
        imapHost: imapHost.trim(),
        smtpHost: smtpHost.trim(),
        username: username.trim() || address.trim(),
      })
      setNotice('Account saved. Its password is kept in this computer’s vault.')
      setName('')
      setAddress('')
      setPassword('')
      setImapHost('')
      setSmtpHost('')
      setUsername('')
      await load()
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
    setBusy(false)
  }

  const removeAccount = async () => {
    if (!local || !account) return
    setBusy(true)
    try {
      await (await local.ready()).call('deleteEmailAccount', { id: account })
      setAccount('')
      setMessages([])
      await load()
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
    setBusy(false)
  }

  const send = async () => {
    if (!local || !to.trim() || (!body.trim() && !subject.trim())) return
    setBusy(true)
    try {
      await (await local.ready()).call(
        'emailSend',
        { account, to: to.trim(), subject: subject.trim(), body },
        60_000,
      )
      setNotice('Email sent.')
      setTo('')
      setSubject('')
      setBody('')
      setError(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
    setBusy(false)
  }

  return (
    <CardSection
      title="Email"
      footer="Codync reads headers over IMAP and sends over SMTP with the account's own servers. Passwords stay in this computer's vault."
    >
      {!running ? (
        <Text size="caption" tone="secondary">Start this computer’s host to use email.</Text>
      ) : (
        <VStack gap={12}>
          {accounts.length ? (
            <VStack gap={8}>
              <HStack gap={8} align="center" wrap>
                <ChoicePicker
                  selection={account}
                  options={accounts.map((item) => ({ id: item.id, label: item.name }))}
                  onChange={setAccount}
                />
                <Button kind="secondary" disabled={busy} onClick={() => void readInbox()}>Read inbox</Button>
                <Button kind="secondary" disabled={busy} onClick={() => void removeAccount()}>Remove account</Button>
              </HStack>
              {messages.length ? (
                <VStack gap={4}>
                  {messages.map((message) => (
                    <VStack key={message.uid} gap={1}>
                      <Text weight="semibold">{message.subject || '(no subject)'}</Text>
                      <Text size="caption" tone="secondary">{message.from} · {message.date}</Text>
                    </VStack>
                  ))}
                </VStack>
              ) : (
                <Text size="caption" tone="secondary">Read the inbox to see the newest messages.</Text>
              )}
            </VStack>
          ) : (
            <Text size="caption" tone="secondary">No email account yet. Add one below.</Text>
          )}

          <VStack gap={6}>
            <Text size="callout" weight="semibold">Add an account</Text>
            <HStack gap={8} align="end" wrap>
              <VStack gap={4} grow><Text size="caption" tone="secondary">Name</Text><Field label="Name" value={name} onChange={setName} placeholder="Work" /></VStack>
              <VStack gap={4} grow><Text size="caption" tone="secondary">Address</Text><Field label="Address" value={address} onChange={setAddress} placeholder="me@example.com" /></VStack>
              <VStack gap={4} grow><Text size="caption" tone="secondary">Password</Text><Field label="Password" value={password} onChange={setPassword} placeholder="App password" secret /></VStack>
            </HStack>
            <HStack gap={8} align="end" wrap>
              <VStack gap={4} grow><Text size="caption" tone="secondary">IMAP server</Text><Field label="IMAP" value={imapHost} onChange={setImapHost} placeholder="imap.example.com" /></VStack>
              <VStack gap={4} grow><Text size="caption" tone="secondary">SMTP server</Text><Field label="SMTP" value={smtpHost} onChange={setSmtpHost} placeholder="smtp.example.com" /></VStack>
              <VStack gap={4} grow><Text size="caption" tone="secondary">Username (optional)</Text><Field label="Username" value={username} onChange={setUsername} placeholder="Defaults to the address" /></VStack>
            </HStack>
            <HStack gap={8} align="center">
              <Text size="caption" tone="secondary" grow>IMAP uses TLS on 993, SMTP STARTTLS on 587.</Text>
              <Button kind="secondary" disabled={busy || !address.trim() || !imapHost.trim() || !smtpHost.trim()} onClick={() => void addAccount()}>Save account</Button>
            </HStack>
          </VStack>

          {accounts.length ? (
            <VStack gap={6}>
              <Text size="callout" weight="semibold">Send an email</Text>
              <HStack gap={8} align="end" wrap>
                <VStack gap={4} grow><Text size="caption" tone="secondary">To</Text><Field label="To" value={to} onChange={setTo} placeholder="someone@example.com" /></VStack>
                <VStack gap={4} grow><Text size="caption" tone="secondary">Subject</Text><Field label="Subject" value={subject} onChange={setSubject} placeholder="Subject" /></VStack>
              </HStack>
              <Field label="Body" value={body} onChange={setBody} placeholder="Write the message" lines={[3, 10]} />
              <HStack gap={8} align="center">
                <Text size="caption" tone="secondary" grow>It is sent immediately from {accounts.find((item) => item.id === account)?.address ?? 'the account'}.</Text>
                <Button kind="secondary" disabled={busy || !to.trim()} onClick={() => void send()}>Send</Button>
              </HStack>
            </VStack>
          ) : null}

          {notice ? <Text size="caption" tone="secondary">{notice}</Text> : null}
          {error ? <Text size="caption" tone="danger">{error}</Text> : null}
        </VStack>
      )}
    </CardSection>
  )
}
