import { useEffect, useState } from 'react'
import { Button, CardForm, CardSection, Switch } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { ModalHeader } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'
import { Field, FormSheet, ItemRow, MarketSection } from './pieces'
import { approvedText, modules, stateText, type HostModule, type ModuleView } from './modules-model'

/**
 * Extensions › Modules: what a person added to Codync beyond what it ships with. Each one is a signed folder the
 * host runs in its own process; nothing it offers a bot runs until the owner approves it here.
 */
export function ModulesPanel({ query }: { query: string }) {
  const store = useStore()
  const tr = useTr()
  const [rows, setRows] = useState<HostModule[] | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState<string | null>(null)
  const [open, setOpen] = useState<HostModule | null>(null)
  const [installing, setInstalling] = useState(false)
  const [folder, setFolder] = useState('')
  const [notice, setNotice] = useState<string | null>(null)

  const apply = (list: HostModule[]) => {
    setRows(list)
    // The open sheet follows its module, so a switch or an approval shows at once.
    setOpen((current) => (current ? list.find((m) => m.id === current.id) ?? null : null))
  }

  const run = async (id: string, work: () => Promise<HostModule[]>) => {
    setBusy(id)
    setError(null)
    try {
      apply(await work())
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : String(failure))
    } finally {
      setBusy(null)
    }
  }

  useEffect(() => {
    let live = true
    void (async () => {
      try {
        const list = await modules.list(await store.ready())
        if (live) setRows(list)
      } catch (failure) {
        if (live) setError(failure instanceof Error ? failure.message : String(failure))
      }
    })()
    return () => {
      live = false
    }
  }, [store])

  const install = async (source: { folder?: string; bundle?: unknown }) => {
    setInstalling(false)
    setNotice(null)
    setBusy('install')
    try {
      apply(await modules.install(await store.ready(), source))
      setFolder('')
    } catch (failure) {
      setNotice(failure instanceof Error ? failure.message : String(failure))
    } finally {
      setBusy(null)
    }
  }

  const shown = (rows ?? []).filter((m) => !query || `${m.name} ${m.description}`.toLowerCase().includes(query.toLowerCase()))

  return (
    <>
      {error ? <Text tone="danger" className="market-channel-note">{error}</Text> : null}
      <MarketSection title={tr('On this computer')}>
        {rows === null ? <Text size="compactSecondary" tone="secondary">{tr('Loading modules…')}</Text> : null}
        {rows !== null && shown.length === 0 ? (
          <Text size="compactSecondary" tone="secondary">
            {tr('No modules yet. A module adds tools and a screen to Codync, signed by whoever made it.')}
          </Text>
        ) : null}
        {shown.map((m) => (
          <ItemRow
            key={m.id}
            title={m.name}
            subtitle={`${m.version ? `v${m.version} · ` : ''}${stateText(m)}${m.description ? ` · ${m.description}` : ''}`}
            icon={<div className="market-tile"><Icon name={m.symbol || 'puzzlepiece.extension'} size={14} /></div>}
            onOpen={() => setOpen(m)}
          />
        ))}
      </MarketSection>
      <HStack gap={8}>
        <Button kind="secondary" onClick={() => setInstalling(true)} disabled={busy !== null}>
          {tr('Install a module')}
        </Button>
      </HStack>
      {notice ? <Text size="caption" tone="secondary">{notice}</Text> : null}

      <FormSheet open={installing} onClose={() => setInstalling(false)} height={520}>
        <ModalHeader title="Install a module" />
        <CardForm>
          <CardSection footer="A module is a folder with a signed manifest: Codync checks the signature and every file before it runs anything.">
            <Field value={folder} onChange={setFolder} placeholder="/Users/you/Modules/weather" label={tr('Module folder')} mono />
            <Button disabled={!folder.trim() || busy !== null} onClick={() => void install({ folder: folder.trim() })}>
              {tr('Install')}
            </Button>
          </CardSection>
          <CardSection title={tr('Or a file')} footer={notice ?? undefined}>
            <label className="market-field">
              <span>{tr('A .module.json another computer saved')}</span>
              <input
                type="file"
                accept=".json,application/json"
                onChange={(event) => {
                  const file = event.target.files?.[0]
                  if (!file) return
                  void file.text().then((text) => {
                    try {
                      void install({ bundle: JSON.parse(text) })
                    } catch {
                      setNotice(tr('That file is not a module bundle.'))
                    }
                  })
                }}
              />
            </label>
          </CardSection>
        </CardForm>
      </FormSheet>

      {open ? (
        <ModuleSheet
          module={open}
          busy={busy === open.id}
          onClose={() => setOpen(null)}
          onRun={run}
          onExport={async () => {
            try {
              const saved = await modules.export(await store.ready(), open.id)
              const url = URL.createObjectURL(new Blob([JSON.stringify(saved.bundle)], { type: 'application/json' }))
              const link = document.createElement('a')
              link.href = url
              link.download = saved.name
              link.click()
              URL.revokeObjectURL(url)
            } catch (failure) {
              setError(failure instanceof Error ? failure.message : String(failure))
            }
          }}
        />
      ) : null}
    </>
  )
}

/** One module: what it asks for, what was approved, its switch, its screens, and taking it off. */
function ModuleSheet({ module: m, busy, onClose, onRun, onExport }: {
  module: HostModule
  busy: boolean
  onClose: () => void
  onRun: (id: string, work: () => Promise<HostModule[]>) => Promise<void>
  onExport: () => Promise<void>
}) {
  const store = useStore()
  const tr = useTr()
  const [view, setView] = useState<ModuleView | null>(null)
  const [viewError, setViewError] = useState<string | null>(null)
  const permissions = m.permissions
  const nothingApproved = m.approved.tools.length === 0 && m.approved.events.length === 0 && m.approved.files === 'none'

  const openView = async (id: string) => {
    setViewError(null)
    try {
      setView(await modules.view(await store.ready(), m.id, id))
    } catch (failure) {
      setViewError(failure instanceof Error ? failure.message : String(failure))
    }
  }

  return (
    <FormSheet open onClose={onClose} height={640}>
      <ModalHeader title={m.name} />
      <CardForm>
        {m.broken ? <CardSection title={tr('It can’t run')}><Text tone="danger" size="compactSecondary">{m.broken}</Text></CardSection> : null}
        <CardSection footer={m.description || undefined}>
          <HStack gap={10}>
            <VStack gap={2}>
              <Text tone="text">{m.version ? `v${m.version}` : m.id}</Text>
              <Text size="caption" tone="secondary">{`${stateText(m)} · ${approvedText(m)}`}</Text>
            </VStack>
            <Switch
              on={m.enabled}
              disabled={busy || m.broken !== null}
              label={tr('On')}
              onChange={(on) => void onRun(m.id, async () => modules.setEnabled(await store.ready(), m.id, on))}
            >
              {tr('On')}
            </Switch>
          </HStack>
          {m.stopped ? <Text size="caption" tone="secondary">{tr('Last stopped: {why}').replace('{why}', m.stopped)}</Text> : null}
        </CardSection>

        {permissions ? (
          <CardSection
            title={tr('What it asks for')}
            footer={nothingApproved
              ? tr('Approve what this module may do. A bot can only call what you approve.')
              : tr('Approved: {what}.').replace('{what}', approvedText(m))}
          >
            <PermissionRows permissions={permissions} approved={m.approved} />
            <HStack gap={8}>
              <Button
                disabled={busy}
                onClick={() => void onRun(m.id, async () => modules.approve(await store.ready(), m.id, null))}
              >
                {nothingApproved ? tr('Approve everything it asks for') : tr('Approve all of it')}
              </Button>
              {!nothingApproved ? (
                <Button
                  kind="secondary"
                  disabled={busy}
                  onClick={() => void onRun(m.id, async () => modules.approve(await store.ready(), m.id, { tools: [], events: [], network: [], files: 'none' }))}
                >
                  {tr('Approve nothing')}
                </Button>
              ) : null}
            </HStack>
          </CardSection>
        ) : null}

        {m.tools.length ? (
          <CardSection title={tr('Tools')} footer={tr('{n} of {all} approved.').replace('{n}', String(m.approved.tools.length)).replace('{all}', String(m.tools.length))}>
            {m.tools.map((tool) => (
              <HStack key={tool.name} gap={8}>
                <Text size="compactSecondary" tone={m.approved.tools.includes(tool.name) ? 'text' : 'secondary'}>{tool.name}</Text>
                <Text size="caption" tone="tertiary">{tool.description}</Text>
              </HStack>
            ))}
          </CardSection>
        ) : null}

        {m.views.length ? (
          <CardSection title={tr('Screens')} footer={viewError ?? undefined}>
            <HStack gap={8}>
              {m.views.map((v) => (
                <Button key={v.id} kind="secondary" disabled={!m.enabled} onClick={() => void openView(v.id)}>{v.title}</Button>
              ))}
            </HStack>
            {view ? (
              <VStack gap={8}>
                {view.sections.map((section) => (
                  <VStack key={section.title} gap={2}>
                    <Text size="compactSecondary" tone="secondary">{section.title}</Text>
                    {section.rows.map((row) => (
                      <HStack key={`${row.label}-${row.value}`} gap={8}>
                        <Text size="compactSecondary" tone="secondary">{row.label}</Text>
                        <Text size="compactSecondary" tone="text">{row.value}</Text>
                      </HStack>
                    ))}
                  </VStack>
                ))}
              </VStack>
            ) : null}
          </CardSection>
        ) : null}

        <CardSection title={tr('This module')} footer={m.folder}>
          <HStack gap={8}>
            <Button kind="secondary" onClick={() => void onExport()}>{tr('Save to a file')}</Button>
            <Button
              kind="secondary"
              disabled={busy}
              onClick={() => void onRun(m.id, async () => modules.uninstall(await store.ready(), m.id))}
            >
              {tr('Remove')}
            </Button>
          </HStack>
          <Text size="caption" tone="tertiary">
            {tr('Removing takes its files off this computer; the bots keep working without its tools.')}
          </Text>
        </CardSection>
      </CardForm>
    </FormSheet>
  )
}

/** Each permission with what the owner approved, in the words a person reads. */
function PermissionRows({ permissions, approved }: { permissions: HostModule['permissions']; approved: HostModule['approved'] }) {
  const tr = useTr()
  if (!permissions) return null
  const line = (label: string, asked: string[], allowed: string[]) => (
    <HStack gap={8}>
      <Text size="compactSecondary" tone="text">{label}</Text>
      <Text size="caption" tone="secondary">
        {asked.length === 0 ? tr('Nothing') : asked.map((item) => `${allowed.includes(item) ? '✓' : '·'} ${item}`).join('  ')}
      </Text>
    </HStack>
  )
  return (
    <VStack gap={6}>
      {line(tr('Tools'), permissions.tools, approved.tools)}
      {line(tr('Events'), permissions.events, approved.events)}
      {line(tr('Hosts'), permissions.network, approved.network)}
      <HStack gap={8}>
        <Text size="compactSecondary" tone="text">{tr('Files')}</Text>
        <Text size="caption" tone="secondary">
          {permissions.files === 'none' ? tr('None') : `${approved.files === 'none' ? '·' : '✓'} ${permissions.files}`}
        </Text>
      </HStack>
    </VStack>
  )
}
