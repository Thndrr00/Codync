import { useEffect, useMemo, useState } from 'react'
import { Button, CardForm, CardSection, TabChoice } from '../../components/Controls'
import { SwitchRow } from './parts'
import { Dialog } from '../../components/Overlay'
import { Text } from '../../components/Text'
import { useModel } from '../../lib/observable'
import { useApp, useStore } from '../../store/context'
import { ComputerPanelStore } from '../../store/computer-panel-store'
import { Field, ErrorText } from '../marketplace/pieces'
import '../marketplace/mcp-tools.css'

type Tab = 'status' | 'files' | 'terminal' | 'activity'
function serviceLabel(value?: string) { return value === 'configured' ? 'configured' : value === 'notConfigured' ? 'not configured' : 'unavailable; unlock host credentials to check' }
export function ComputerPanel({ botId }: { botId: string }) {
  const store = useStore()
  const app = useApp()
  const model = useModel(useMemo(() => new ComputerPanelStore(botId, () => store.ready()), [store, botId]))
  const [tab, setTab] = useState<Tab>('status')
  const [path, setPath] = useState('')
  const [command, setCommand] = useState('')
  const [site, setSite] = useState('')
  const [login, setLogin] = useState('')
  const [code, setCode] = useState('')
  const [confirm, setConfirm] = useState<{ model: ComputerPanelStore; command: string } | null>(null)
  useEffect(() => {
    setTab('status')
    setPath('')
    setCommand('')
    setSite('')
    setLogin(''); setCode('')
    setConfirm(null)
    model.resume()
    void model.load()
    const timer = setInterval(() => void model.load(), 5_000)
    return () => { clearInterval(timer); model.dispose() }
  }, [model])
  const state = model.state
  const canFiles = !!state?.enabled && state.permissions.files
  const canShell = !!state?.enabled && state.permissions.shell
  return <CardForm>
    <TabChoice label="Computer panel" selection={tab} onChange={setTab} options={[{ id: 'status', label: 'Status' }, { id: 'files', label: 'Files' }, { id: 'terminal', label: 'Terminal' }, { id: 'activity', label: 'Activity' }]} />
    <Button kind="secondary" disabled={model.loading} onClick={() => void model.load(true)}>Refresh computer</Button>
    {model.loading && !state && <Text as="div" role="status">Loading computer…</Text>}
    {tab === 'status' && state && <CardSection title="Computer permissions" footer="These control Codync's tools on the real host. The desktop permission covers apps, mouse and keyboard; it is not a browser sandbox. Harness tools have their own permissions. Revocation cancels pending work and owned shell processes; effects already sent cannot be undone.">
      <Text tone="secondary">Workspace: {state.workspace}</Text>
      <Text>{state.screen.userControl ? 'You control the screen' : state.screen.agentBot ? `Bot ${state.screen.agentBot} is using the screen` : 'No bot using the screen'}</Text>
      <SwitchRow title="Use the computer" on={state.enabled} onChange={(enabled) => void model.setPermissions(enabled, state.permissions)} disabled={model.busy} />
      {(['browser', 'files', 'shell'] as const).map((capability) => <SwitchRow key={capability} title={{ browser: 'Desktop and browser', files: 'Workspace files', shell: 'Shell commands' }[capability]}
        on={state.permissions[capability]} disabled={model.busy || !state.enabled} onChange={(enabled) => void model.setPermissions(state.enabled, { ...state.permissions, [capability]: enabled })} />)}
        <Text weight="semibold">Connected services</Text>
        <Text tone="secondary">Research: {serviceLabel(state.services?.research)} · Slack: {serviceLabel(state.services?.slack)}. Configure research in Windows Settings; Slack setup is local to the host.</Text>
        <Text weight="semibold">Passwords and codes</Text>
      <Text tone="secondary">Pause Codync computer tools and stop current bot work, then type the secret yourself on the real host. Verify the website first. Never paste the secret into chat. This does not isolate independent harness tools.</Text>
      {state.screen.protectedEntry ? <>
        <Text>Manual entry active: {state.screen.protectedEntry.site}</Text>
        {state.protectedBrowserAvailable && app.local === store && window.codync.platform === 'win32' ? <>
          <Text tone="secondary">Open an isolated private browser, navigate to the sign-in page on this exact origin and focus its password or one-time-code input. Saved passwords are resolved on the host. Codes go directly to the host, never into chat. Finishing closes the browser by default. Keeping the sign-in session closes every previous private page and opens a blank window; if that cannot be verified, the browser closes instead.</Text>
          <Button disabled={model.busy} onClick={() => void model.protectedBrowser('openProtectedBrowser', state.screen.protectedEntry!.id)}>Open protected browser</Button>
          <Field label="Saved login ID" value={login} onChange={setLogin} placeholder="ID from saved website logins" />
          <Button disabled={model.busy || !login.trim()} onClick={() => void model.protectedBrowser('fillProtectedLogin', state.screen.protectedEntry!.id, login.trim())}>Fill saved password in focused field</Button>
          <Field label="One-time code" secret value={code} onChange={setCode} placeholder="Code for the approved website" />
          <Button disabled={model.busy || !code.trim()} onClick={() => { const value = code; setCode(''); void model.protectedBrowser('fillProtectedCode', state.screen.protectedEntry!.id, value) }}>Fill code in focused field</Button>
          <Button kind="secondary" disabled={model.busy} onClick={() => void model.protectedEntry(null, state.screen.protectedEntry!.id, true)}>Keep sign-in session and hand back</Button>
        </> : <Text tone="secondary">Protected browser autofill is available only from the local Windows owner session.</Text>}
        <Button disabled={model.busy} onClick={() => void model.protectedEntry(null, state.screen.protectedEntry?.id)}>Finish entry and close protected browser</Button>
      </> : <>
        <Field label="Website for manual entry" value={site} onChange={setSite} placeholder="https://example.com" />
        <Button disabled={model.busy || !state.enabled || !state.permissions.browser || !site.trim()} onClick={() => void model.protectedEntry(site)}>Pause for manual entry</Button>
      </>}
    </CardSection>}
    {tab === 'files' && <CardSection title="Workspace files" footer="Relative paths only. Preview supports UTF-8 text up to 128 KiB. Links cannot be opened from the file list.">
      {!canFiles && <Text tone="secondary">Enable computer access and Workspace files in Status.</Text>}
      <Field label="Workspace path" value={path} onChange={setPath} placeholder="Folder relative to workspace" />
      <Button disabled={!canFiles} onClick={() => void model.open(path)}>Open folder</Button>
      <Text tone="secondary">Folder: {model.path || '/'}</Text>
      {model.files.map((file) => <Button key={file.name} kind="secondary" disabled={!canFiles || file.link} onClick={() => { const next = [model.path, file.name].filter(Boolean).join('/'); if (file.directory) setPath(next); void model.open(next, file.directory) }}>{file.directory ? 'Folder: ' : ''}{file.name}{file.link ? ' (link)' : ''}</Button>)}
      {canFiles && !model.files.length && <Text tone="secondary">Open a folder to list its files. Empty folders show no entries.</Text>}
      {model.truncated && <Text tone="secondary">Showing at most 500 entries. Open a more specific folder.</Text>}
      {model.fileText !== null && <pre className="mcp-tool-arguments">{model.fileText}</pre>}
    </CardSection>}
    {tab === 'terminal' && <CardSection title="Workspace terminal" footer="Each command runs separately in the workspace on the real host. No interactive stdin. Limit: 30 seconds and 64 KiB per output stream. A network failure can leave the outcome unknown; inspect before retrying.">
      {!canShell && <Text tone="secondary">Enable computer access and Shell commands in Status.</Text>}
      <Field label="Command" value={command} onChange={setCommand} placeholder="Command to run on this host" lines={[2, 6]} />
      <Button disabled={!canShell || model.running || !command.trim()} onClick={() => setConfirm({ model, command })}>{model.running ? 'Running…' : 'Run command'}</Button>
      {model.running && <Text as="div" role="status">Running. Turn off Shell commands in Status to cancel.</Text>}
      {model.output && <><Text>Exit code: {model.output.exitCode ?? 'unavailable'}</Text><pre className="mcp-tool-arguments">{model.output.stdout}{model.output.stderr}</pre></>}
    </CardSection>}
    {tab === 'activity' && <CardSection title="Computer activity" footer="Metadata only; commands, typed values, file contents and results are not retained in this history.">
      {!state?.activity.length && <Text tone="secondary">No computer activity recorded.</Text>}
      {state?.activity.map((item) => <Text as="div" key={item.id}>{item.action} · {item.result} · {new Date(item.createdAt).toLocaleString()}</Text>)}
    </CardSection>}
    {model.error && <ErrorText text={model.error} />}
    {model.notice && <Text as="div" role="status">{model.notice}</Text>}
    <Dialog open={confirm?.model === model} title="Run on the real host?" message={confirm?.command} onClose={() => setConfirm(null)} actions={[{ title: 'Run command', action: () => { if (confirm?.model === model) void model.run(confirm.command) } }]} />
  </CardForm>
}
