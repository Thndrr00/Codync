import { useEffect, useMemo, useState } from 'react'
import { Button, CardSection, Switch } from '../../components/Controls'
import { Dialog, ModalHeader, Sheet, SheetPage } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { githubAppIdOk, githubRepoOk, githubWebhookSecretOk, type GitHubRepoMap } from '../../../shared/github'
import { useTr } from '../../lib/use-language'
import { useModel } from '../../lib/observable'
import { useApp } from '../../store/context'
import { GitHubAppStore } from '../../store/github-app-store'
import { ErrorText, Field } from '../marketplace/pieces'

const MANIFEST = 'https://codync.com/docs/github-app#creating-the-app'

/** The GitHub App whose signed deliveries reach the mapped bots' routine triggers. */
export function GitHubAppSection() {
  const tr = useTr()
  const app = useApp()
  const local = app.local
  const model = useModel(useMemo(() => new GitHubAppStore(async () => {
    if (!local) throw new Error('Start this computer’s host to connect a GitHub App.')
    return local.ready()
  }), [local]))
  const [appId, setAppId] = useState('')
  const [privateKey, setPrivateKey] = useState('')
  const [secret, setSecret] = useState('')
  const [slug, setSlug] = useState('')
  const [editing, setEditing] = useState(false)
  const [mapping, setMapping] = useState<string | null>(null)
  const [unmapping, setUnmapping] = useState<string | null>(null)
  useEffect(() => { model.resume(); void model.load(); return () => model.dispose() }, [model])
  const status = model.status
  const bots = (local?.roster ?? []).filter((b) => b.kind !== 'group' && b.backend !== 'external').map((b) => ({ id: b.id, label: b.name }))
  const ready = githubAppIdOk(appId) && privateKey.includes('PRIVATE KEY') && githubWebhookSecretOk(secret)
  const save = async () => {
    if (await model.register(appId.trim(), privateKey.trim(), secret.trim(), slug.trim())) {
      setAppId(''); setPrivateKey(''); setSecret(''); setSlug(''); setEditing(false)
    }
  }
  const mapped = (repo: string) => status?.repos.find((m) => m.repo === repo)

  return (
    <CardSection
      title="GitHub App"
      footer="An App of your own signs its deliveries, so a routine runs on pull requests, reviews, checks and issues without a person's token. Required permissions: contents and pull requests read, checks read. Events: pull request, pull request review, check run, issues. The key and the webhook secret stay in this computer's credential vault."
    >
      {status?.configured && !editing ? (
        <VStack gap={6} className="settings-stack">
          <Text>{tr('App')} {status.appId}</Text>
          <Text size="caption" tone="tertiary">
            {status.installations.length === 0 ? tr('Not installed anywhere yet.') : `${status.installations.length} ${tr('installations')}`}
          </Text>
          <HStack gap={8}>
            <Button kind="secondary" onClick={() => window.codync.app.openExternal(`${MANIFEST}&app_id=${status.appId}`)}>{tr('Open manifest')}</Button>
            {status.slug ? <Button kind="secondary" onClick={() => window.codync.app.openExternal(`https://github.com/apps/${status.slug}/installations/new`)}>{tr('Install on repositories')}</Button> : null}
            <Button kind="secondary" disabled={model.busy} onClick={() => void model.refresh()}>{tr('Refresh installations')}</Button>
          </HStack>
          {status.installations.map((installation) => (
            <VStack key={installation.id} gap={4} className="settings-stack">
              <Text size="caption" tone="secondary">{installation.account}</Text>
              {installation.repos.map((repo) => (
                <div key={repo} className="settings-row">
                  <Text grow>{repo}</Text>
                  <Text size="caption" tone="tertiary">{botNames(mapped(repo), bots)}</Text>
                  <Button kind="secondary" disabled={model.busy || bots.length === 0} onClick={() => setMapping(repo)}>{tr('Map bots')}</Button>
                  {mapped(repo) ? <Button kind="secondary" disabled={model.busy} onClick={() => setUnmapping(repo)}>{tr('Unmap')}</Button> : null}
                </div>
              ))}
              {installation.repos.length === 0 ? <Text size="caption" tone="tertiary">{tr('No repositories chosen for this installation.')}</Text> : null}
            </VStack>
          ))}
          <HStack gap={8}>
            <Button kind="secondary" disabled={model.busy} onClick={() => setEditing(true)}>{tr('Replace the App')}</Button>
            <Button kind="secondary" disabled={model.busy} onClick={() => void model.disconnect()}>{tr('Disconnect the App')}</Button>
          </HStack>
        </VStack>
      ) : (
        <VStack gap={6} className="settings-stack">
          <Text size="caption" tone="tertiary">
            {tr('Create the App from GitHub’s manifest flow, then paste what it gives you: the numeric id, the private key and the webhook secret.')}
          </Text>
          <Field value={appId} onChange={setAppId} placeholder="123456" label="App id" mono />
          <Field value={privateKey} onChange={setPrivateKey} placeholder="-----BEGIN RSA PRIVATE KEY-----" label="Private key" lines={[3, 10]} mono />
          <Field secret value={secret} onChange={setSecret} placeholder="At least 16 visible characters" label="Webhook secret" />
          <Field value={slug} onChange={setSlug} placeholder="my-codync-app" label="App slug (for the install link)" />
          <HStack gap={8}>
            <Button disabled={!ready || model.busy} onClick={() => void save()}>{model.busy ? tr('Checking…') : tr('Connect the App')}</Button>
            {status?.configured ? <Button kind="secondary" disabled={model.busy} onClick={() => setEditing(false)}>{tr('Cancel')}</Button> : null}
          </HStack>
        </VStack>
      )}
      {model.error ? <ErrorText text={model.error} /> : null}
      <Sheet open={mapping !== null} onClose={() => setMapping(null)} width={380}>
        {mapping ? <MappingSheet repo={mapping} bots={bots} model={model} close={() => setMapping(null)} /> : null}
      </Sheet>
      <Dialog
        open={unmapping !== null}
        title={tr('Unmap this repository?')}
        message={tr('Its deliveries stop reaching the bots until you map it again.')}
        actions={[{ title: tr('Unmap'), action: () => { const repo = unmapping; setUnmapping(null); if (repo) void model.unmap(repo) } }]}
        onClose={() => setUnmapping(null)}
      />
    </CardSection>
  )
}

function botNames(map: GitHubRepoMap | undefined, bots: { id: string; label: string }[]) {
  const tr = useTr()
  if (!map) return tr('Not mapped')
  const names = map.bots.map((id) => bots.find((b) => b.id === id)?.label ?? id).join(', ')
  const branches = map.branches.length > 0 ? ` · ${map.branches.join(', ')}` : ''
  return names + branches
}

/** Which bots one repository reaches, and on which branches (empty for every branch). */
function MappingSheet({ repo, bots, model, close }: {
  repo: string
  bots: { id: string; label: string }[]
  model: GitHubAppStore
  close: () => void
}) {
  const tr = useTr()
  const existing = model.status?.repos.find((m) => m.repo === repo)
  const [chosen, setChosen] = useState<string[]>(existing?.bots ?? [])
  const [branches, setBranches] = useState((existing?.branches ?? []).join(', '))
  const toggle = (id: string) => setChosen((ids) => ids.includes(id) ? ids.filter((i) => i !== id) : [...ids, id])
  const save = async () => {
    const list = branches.split(/[\s,]+/).filter(Boolean)
    if (await model.map(repo, chosen, list)) close()
  }
  return (
    <SheetPage>
      <ModalHeader title={repo} subtitle={tr('Runs its events on the bots you pick.')} onClose={close} />
      <VStack gap={4} className="settings-stack">
        {bots.map((bot) => (
          <Switch key={bot.id} on={chosen.includes(bot.id)} onChange={() => toggle(bot.id)}>{bot.label}</Switch>
        ))}
        {bots.length === 0 ? <Text size="caption" tone="tertiary">{tr('Create a bot first.')}</Text> : null}
        <Field value={branches} onChange={setBranches} placeholder="main, release/*" label="Branches (empty for all)" />
        <HStack>
          <Button disabled={chosen.length === 0 || !githubRepoOk(repo) || model.busy} onClick={() => void save()}>{tr('Save mapping')}</Button>
        </HStack>
        {model.error ? <ErrorText text={model.error} /> : null}
      </VStack>
    </SheetPage>
  )
}
