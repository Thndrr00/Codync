import { useState } from 'react'
import { bytesText } from '@shared/health'
import { installLine, installing } from '@shared/tools'
import { Button, CardForm, CardSection, Spinner, TabChoice } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { ModalHeader, SheetPage } from '../../components/Overlay'
import { useTool } from '../../lib/use-tool'
import { useTr } from '../../lib/use-language'
import { ErrorText } from './pieces'
import { ComfyJobs } from './comfy/ComfyJobs'
import { ComfyModels } from './comfy/ComfyModels'
import { ComfyTemplates } from './comfy/ComfyTemplates'

type Tab = 'templates' | 'models' | 'jobs' | 'settings'

/**
 * ComfyUI's card in Extensions › Apps: the gallery of named templates and their forms, the model
 * library, the jobs with their real progress, and the tool itself (start, stop, log, space). The
 * host owns it all; the card only asks and shows.
 */
export function ComfySheet() {
  const tr = useTr()
  const [tab, setTab] = useState<Tab>('templates')
  const { state, hostKnown, call, setState } = useTool('comfy')
  const [error, setError] = useState<string | null>(null)
  const card = state?.comfy
  const job = state?.job ?? null
  const running = state?.state === 'running'
  const active = running || state?.state === 'starting'

  const act = (run: () => Promise<unknown>) => {
    setError(null)
    void run().catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
  }

  return (
    <SheetPage>
      <ModalHeader title="ComfyUI" subtitle={tr('Make images, video, audio and 3D on this computer. Bots can use it when you turn it on for them.')} />
      <TabChoice
        label="ComfyUI sections"
        selection={tab}
        options={[
          { id: 'templates', label: 'Templates' },
          { id: 'models', label: 'Models' },
          { id: 'jobs', label: 'Jobs' },
          { id: 'settings', label: 'Settings' },
        ]}
        onChange={setTab}
      />
      <CardForm>
        {tab === 'templates' ? <ComfyTemplates call={call} onQueued={() => setTab('jobs')} /> : null}
        {tab === 'models' ? <ComfyModels call={call} /> : null}
        {tab === 'jobs' ? <ComfyJobs call={call} current={job} /> : null}
        {tab === 'settings' ? (
          <CardSection>
            <VStack gap={10}>
              {!hostKnown || state === null ? (
                <HStack gap={8}><Spinner size={18} /><Text tone="secondary">{tr('Checking {name}…').replace('{name}', 'ComfyUI')}</Text></HStack>
              ) : card && !card.bundled ? (
                <Text tone="secondary">{tr('This build of Codync does not include ComfyUI.')}</Text>
              ) : (
                <>
                  <Text tone="text" weight="medium">
                    {running ? tr('Running') : state.state === 'starting' ? tr('Starting {name}… the first start can take a few minutes.').replace('{name}', 'ComfyUI') : state.state === 'error' ? tr('Stopped with an error') : tr('Stopped')}
                  </Text>
                  {card ? (
                    <Text size="caption" tone="secondary">
                      {tr('Version {version} · {size} used').replace('{version}', card.version).replace('{size}', bytesText(card.usedBytes))}
                      {running ? ` · ${tr('Port {port}').replace('{port}', String(state.port))}` : ''}
                    </Text>
                  ) : null}
                  {card?.notice ? <Text size="caption" tone="attention">{tr(card.notice)}</Text> : null}
                  {state.error ? <Text size="caption" tone="attention">{state.error}</Text> : null}
                  {installing(state.install) ? (
                    <HStack gap={8}><Spinner size={14} /><Text size="caption" tone="secondary">{installLine('ComfyUI', state.install, tr)}</Text></HStack>
                  ) : null}
                  {state.install?.phase === 'failed' && state.install.error ? <Text size="caption" tone="attention">{state.install.error}</Text> : null}
                  {card && !card.installed && !installing(state.install) ? (
                    <Text size="caption" tone="secondary">
                      {tr('{name} downloads about {size} the first time it starts.').replace('{name}', 'ComfyUI').replace('{size}', bytesText(card.downloadBytes))}
                    </Text>
                  ) : null}
                  <HStack gap={8}>
                    {running ? (
                      <Button onClick={() => act(() => window.codync.tools.open('comfy'))}>{tr('Open')}</Button>
                    ) : (
                      <Button disabled={active || installing(state.install)} onClick={() => act(() => call('toolStart').then(setState))}>
                        {card && !card.installed ? tr('Download and start') : tr('Start')}
                      </Button>
                    )}
                    <Button kind="secondary" disabled={!state.enabled && !active} onClick={() => act(() => call('toolStop').then(setState))}>{tr('Stop')}</Button>
                    <Button kind="secondary" onClick={() => act(() => window.codync.tools.openLog('comfy'))}>{tr('Log')}</Button>
                    <Button kind="secondary" onClick={() => act(() => window.codync.tools.openModels('comfy'))}>{tr('Models folder')}</Button>
                  </HStack>
                  <Text size="caption" tone="secondary">
                    {tr('Templates, models and jobs stay in {dir}; nothing leaves this computer.')
                      .replace('{dir}', card?.dataDir ?? '')}
                  </Text>
                </>
              )}
              {error ? <ErrorText text={error} /> : null}
            </VStack>
          </CardSection>
        ) : null}
      </CardForm>
    </SheetPage>
  )
}
