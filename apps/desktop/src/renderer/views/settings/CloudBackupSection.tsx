import { useEffect, useMemo, useState } from 'react'
import { Button, CardSection, ChoicePicker, Switch } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useModel } from '../../lib/observable'
import { useApp } from '../../store/context'
import { Field } from '../marketplace/pieces'

interface BackupFile {
  name: string
  at: number
}

interface BackupStatus {
  url: string
  username: string
  backend: string
  rclone: boolean
  enabled: boolean
  keep: number
  lastAt?: number
  lastError?: string
  configured: boolean
  hasPassword: boolean
  files: BackupFile[]
}

const when = (ms?: number) =>
  ms ? new Date(ms).toLocaleString(undefined, { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' }) : null

/** An encrypted copy of this computer's database in any WebDAV folder (Nextcloud, Box, pCloud). */
export function CloudBackupSection() {
  const app = useApp()
  const local = useModel(app.local)
  const running = local !== null && local.connection.kind === 'online'
  const [status, setStatus] = useState<BackupStatus | null>(null)
  const [url, setUrl] = useState('')
  const [username, setUsername] = useState('')
  const [backend, setBackend] = useState('webdav')
  const [rclone, setRclone] = useState(false)
  const [password, setPassword] = useState('')
  const [keep, setKeep] = useState('14')
  const [enabled, setEnabled] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)

  const load = useMemo(() => async () => {
    if (!local) return
    try {
      const client = await local.ready()
      const next = await client.call<BackupStatus>('cloudBackupStatus', {})
      setStatus(next)
      setUrl(next.url)
      setUsername(next.username)
      setBackend(next.backend || 'webdav')
      setRclone(next.rclone)
      setKeep(String(next.keep))
      setEnabled(next.enabled)
      setError(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
  }, [local])

  useEffect(() => {
    if (running) void load()
  }, [running, load])

  const save = async (runNow: boolean) => {
    if (!local) return
    const count = Number.parseInt(keep, 10)
    if (!Number.isFinite(count) || count < 1 || count > 100) {
      setError('Keep between 1 and 100 backups.')
      return
    }
    setBusy(true)
    setNotice(null)
    try {
      const client = await local.ready()
      await client.call('saveCloudBackup', {
        url: url.trim(),
        username: username.trim(),
        password,
        backend,
        enabled,
        keep: count,
      })
      setPassword('')
      if (runNow) {
        const answer = await client.call<{ name: string; bytes: number }>('runCloudBackup', {}, 300_000)
        setNotice(`Uploaded ${answer.name} (${Math.round(answer.bytes / 1024).toLocaleString()} KB).`)
      } else {
        setNotice('Saved.')
      }
      await load()
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
    setBusy(false)
  }

  return (
    <CardSection
      title="Cloud backup"
      footer="Codync copies its database, seals it with this computer's vault key and uploads it to your own WebDAV folder. The server only sees ciphertext."
    >
      {!running ? (
        <Text size="caption" tone="secondary">Start this computer’s host to use cloud backup.</Text>
      ) : (
        <VStack gap={12}>
          <VStack gap={6}>
            <Text size="callout" weight="semibold">Where</Text>
            <ChoicePicker
              selection={backend}
              options={[{ id: 'webdav', label: 'WebDAV folder' }, { id: 'rclone', label: 'rclone remote' }]}
              onChange={setBackend}
            />
            <Text size="caption" tone="secondary">
              {backend === 'rclone'
                ? (rclone ? 'A remote you configured in rclone on this computer, like gdrive:Codync.' : 'rclone is not on this computer’s PATH. Install it and configure a remote first.')
                : 'A WebDAV folder, like Nextcloud, ownCloud, Box or pCloud.'}
            </Text>
            <Field
              label={backend === 'rclone' ? 'rclone path' : 'WebDAV folder'}
              value={url}
              onChange={setUrl}
              placeholder={backend === 'rclone' ? 'gdrive:Codync' : 'https://cloud.example.com/remote.php/dav/files/me/Codync'}
            />
            {backend === 'webdav' ? (
              <HStack gap={8} align="end" wrap>
                <VStack gap={4} grow><Text size="caption" tone="secondary">Username</Text><Field label="Username" value={username} onChange={setUsername} placeholder="me" /></VStack>
                <VStack gap={4} grow><Text size="caption" tone="secondary">Password</Text><Field label="Password" value={password} onChange={setPassword} placeholder={status?.hasPassword ? 'Saved; leave empty to keep it' : 'App password'} secret /></VStack>
                <VStack gap={4}><Text size="caption" tone="secondary">Keep</Text><Field label="Keep" value={keep} onChange={setKeep} placeholder="14" /></VStack>
              </HStack>
            ) : (
              <HStack gap={8} align="end" wrap>
                <VStack gap={4}><Text size="caption" tone="secondary">Keep</Text><Field label="Keep" value={keep} onChange={setKeep} placeholder="14" /></VStack>
              </HStack>
            )}
            <Switch on={enabled} onChange={setEnabled} label="Back up every day">Back up every day</Switch>
            <Text size="caption" tone="secondary">A backup is uploaded only while this computer’s host runs. The file is sealed before it leaves.</Text>
          </VStack>

          <HStack gap={8} align="center">
            <Button kind="secondary" disabled={busy || !url.trim()} onClick={() => void save(false)}>Save</Button>
            <Button disabled={busy || !status?.configured} onClick={() => void save(true)}>Back up now</Button>
            {busy ? <Text size="caption" tone="secondary">Working…</Text> : null}
          </HStack>

          {status?.lastAt ? <Text size="caption" tone="secondary">Last backup: {when(status.lastAt)}</Text> : <Text size="caption" tone="secondary">No backup yet.</Text>}
          {status?.lastError ? <Text size="caption" tone="danger">{status.lastError}</Text> : null}
          {status && status.files.length ? (
            <VStack gap={4}>
              <Text size="caption" weight="semibold" tone="secondary">Uploaded</Text>
              {status.files.slice().reverse().map((file) => (
                <HStack key={file.name} gap={8}>
                  <Text size="caption" grow>{file.name}</Text>
                  <Text size="caption" tone="tertiary">{when(file.at)}</Text>
                </HStack>
              ))}
            </VStack>
          ) : null}

          {notice ? <Text size="caption" tone="secondary">{notice}</Text> : null}
          {error ? <Text size="caption" tone="danger">{error}</Text> : null}
        </VStack>
      )}
    </CardSection>
  )
}
