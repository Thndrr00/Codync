import { useEffect, useState, type ReactNode } from 'react'
import { islandApprovalAge, type IslandIntegrations } from '@shared/island-integrations'
import { Button, ChipChoice } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { useModel } from '../../lib/observable'
import { useTr } from '../../lib/use-language'
import type { IslandModel } from './island-model'
import { IslandPanel } from './IslandPanel'
import { OperationsView } from './IslandOperations'
import './island-integrations.css'

type IntegrationTab = 'project' | 'work' | 'tools' | 'operations'
type NoteRequest = { ref: string; text: string; requestId: string }

/** Existing product workflows, grouped without adding more buttons to the island toolbar. */
export function IntegrationsView({ model }: { model: IslandModel }) {
  const tr = useTr()
  useModel(model.dictation)
  const [tab, setTab] = useState<IntegrationTab>('project')
  const [notes, setNotes] = useState<Record<string, string>>({})
  const [submitted, setSubmitted] = useState<NoteRequest | null>(null)
  const selected = model.focus?.ref
  const data = model.summary.integrations?.ref === selected ? model.summary.integrations : null
  const text = selected ? notes[selected] ?? '' : ''

  useEffect(() => { if (selected) model.action({ kind: 'loadIntegrations', ref: selected }) }, [model, selected])
  useEffect(() => {
    if (!submitted || data?.ref !== submitted.ref || data.savedNote !== submitted.requestId) return
    setNotes((current) => current[submitted.ref] === submitted.text ? { ...current, [submitted.ref]: '' } : current)
    setSubmitted(null)
  }, [data?.savedNote, data?.ref, submitted])

  return <IslandPanel>
    <div className="island-control-heading"><strong>{tr('Integrations')}</strong><button className="island-tab" title={tr('Refresh integrations')} aria-label={tr('Refresh integrations')} disabled={!selected || data?.loading} onClick={() => selected && model.action({ kind: 'loadIntegrations', ref: selected })}><Icon name="arrow.clockwise" size={13} /></button></div>
    <ChipChoice<IntegrationTab> label="Integration section" selection={tab} options={[{ id: 'project', label: 'Project' }, { id: 'work', label: 'Work' }, { id: 'tools', label: 'Tools' }, { id: 'operations', label: 'Operations' }]} onChange={setTab} />
    {model.summary.bots.length ? <ChipChoice label="Select bot" selection={selected ?? ''} options={model.summary.bots.map((bot) => ({ id: bot.ref, label: bot.name }))} onChange={(ref) => model.selectFocus(ref)} /> : <span className="island-subtitle">{tr('No bots yet')}</span>}
    {selected && !data ? <span className="island-subtitle" role="status">{tr('Loading integrations…')}</span> : null}
    {data?.loading ? <span className="island-subtitle" role="status">{tr('Refreshing integrations…')}</span> : null}
    {data?.fetchedAt ? <time className="island-subtitle">{tr('Updated')} · {new Date(data.fetchedAt).toLocaleTimeString()}</time> : null}
    {data?.errors.find((error) => error.section === 'connection') ? <span className="island-control-error" role="status">{data.errors.find((error) => error.section === 'connection')!.message}</span> : null}
    {data?.actionError ? <span className="island-control-error" role="alert">{data.actionError}</span> : null}
    {tab === 'operations' ? <OperationsView model={model} /> : null}
    {tab === 'project' && data ? <ProjectIntegrations model={model} data={data} /> : null}
    {tab === 'work' ? <WorkIntegrations model={model} data={data} /> : null}
    {tab === 'tools' ? <>
      <ToolIntegrations model={model} data={data} />
      {data ? <section className="island-integration-section" aria-label={tr('Quick note')}>
        <strong>{tr('Quick note')}</strong>
        <label className="island-note-label"><span className="island-subtitle">{tr('Append to this bot’s scratchpad')}</span><textarea className="island-search-input island-note-input" value={text} maxLength={4000} rows={3} onChange={(event) => setNotes({ ...notes, [data.ref]: event.target.value })} /></label>
        <div className="island-control-actions"><Button disabled={data.busy || !text.trim()} onClick={() => {
          const request = { ref: data.ref, text: text.trim(), requestId: crypto.randomUUID() }
          setSubmitted({ ...request, text })
          model.action({ kind: 'appendScratchpad', ...request })
        }}>{tr('Save note')}</Button><Button kind="secondary" disabled={!text} onClick={() => setNotes({ ...notes, [data.ref]: '' })}>{tr('Clear')}</Button></div>
        {data.savedNote ? <span className="island-subtitle" role="status">{tr('Note saved')}</span> : null}
      </section> : null}
    </> : null}
  </IslandPanel>
}

function IntegrationSection({ data, section, title, empty, children, hasItems }: { data: IslandIntegrations; section: string; title: string; empty: string; children: ReactNode; hasItems: boolean }) {
  const tr = useTr()
  const error = data.errors.find((error) => error.section === section)
  const disconnected = data.errors.some((error) => error.section === 'connection')
  return <section className="island-integration-section" aria-label={tr(title)}>
    <strong>{tr(title)}</strong>
    {error ? <span className="island-control-error" role="status">{tr('Unavailable')} · {error.message}</span> : null}
    {hasItems ? children : disconnected ? <span className="island-subtitle">{tr('Unavailable')}</span> : !error ? <span className="island-subtitle">{tr(data.loading ? 'Loading…' : empty)}</span> : null}
  </section>
}

function ProjectIntegrations({ model, data }: { model: IslandModel; data: IslandIntegrations }) {
  const tr = useTr()
  const github = data.github
  const open = (url: string) => model.action({ kind: 'openGitHubLink', ref: data.ref, url })
  return <>
    <IntegrationSection data={data} section="github" title="GitHub" empty="GitHub status unavailable" hasItems={!!github}>
      {github ? <>
        <span className="island-subtitle">{github.repository ?? tr('No linked repository')}{github.branch ? ` · ${github.branch}` : ''}</span>
        {!github.configured ? <span className="island-subtitle">{tr('Connect GitHub in Extensions to see project activity.')}</span> : null}
        <strong>{tr('Pull requests and reviews')}</strong>
        {github.pulls.length ? github.pulls.map((pull) => <div className="island-integration-item" key={pull.number}>
          <button className="island-control-row" onClick={() => open(pull.url)}><span>#{pull.number} · {pull.title}</span><Icon name="arrow.up.right.square" size={12} /></button>
          <span className="island-subtitle">{tr(pull.state)}{pull.draft ? ` · ${tr('Draft')}` : ''}</span>
          <span className="island-subtitle">{tr('Review requested')}: {pull.reviewers.join(', ') || tr('None reported')}</span>
          {pull.reviews.map((review, index) => <span className="island-subtitle" key={`${review.login}/${index}`}>{review.login} · {tr(review.state)}</span>)}
          <strong>{tr('Head commit checks')}</strong>
          {pull.checks.length ? pull.checks.map((check, index) => check.url ? <button key={`${check.name}/${index}`} className="island-control-row" onClick={() => open(check.url!)}><span>{check.name}</span><span>{tr(check.conclusion ?? check.status)}</span></button> : <div key={`${check.name}/${index}`} className="island-control-row"><span>{check.name}</span><span>{tr(check.conclusion ?? check.status)}</span></div>) : <span className="island-subtitle">{tr(github.error ? 'Checks unavailable' : 'No checks reported')}</span>}
        </div>) : !github.error && github.configured && github.repository ? <span className="island-subtitle">{tr('No matching pull requests')}</span> : null}
        <strong>{tr('Recent assigned issues')}</strong>
        {github.issues.length ? github.issues.map((issue) => <div key={issue.number} className="island-integration-item"><button className="island-control-row" onClick={() => open(issue.url)}><span>#{issue.number} · {issue.title}</span><Icon name="arrow.up.right.square" size={12} /></button><span className="island-subtitle">{tr(issue.state)} · {issue.assignees.join(', ') || tr('Unassigned')}</span></div>) : !github.error && github.configured && github.repository ? <span className="island-subtitle">{tr('No assigned issues')}</span> : null}
      </> : null}
    </IntegrationSection>
    <IntegrationSection data={data} section="upcoming" title="Upcoming routines" empty="No upcoming routines reported" hasItems={!!data.upcoming.length}>
      {data.upcoming.map((routine) => <button key={`${routine.id}/${routine.at}`} className="island-control-row" onClick={() => model.action({ kind: 'openFlow', ref: data.ref })}><span>{routine.name}</span><time>{new Date(routine.at).toLocaleString()}</time></button>)}
    </IntegrationSection>
  </>
}

function WorkIntegrations({ model, data }: { model: IslandModel; data: IslandIntegrations | null }) {
  const tr = useTr()
  const selected = model.focus?.ref
  const signals = model.summary.signals
  const uploads = signals?.uploads.filter((upload) => !selected || upload.ref === selected) ?? []
  const approvals = model.summary.permissions.filter((approval) => !selected || approval.ref === selected)
  const computer = data?.ref.split('/')[0]
  const name = (id: string) => model.summary.bots.find((bot) => bot.ref === `${computer}/${id}`)?.name ?? id
  return <>
    {data ? <>
      <IntegrationSection data={data} section="terminals" title="Agent setup terminals" empty="No retained setup terminals" hasItems={!!data.terminals.length}>
        {data.terminals.map((terminal) => <button key={terminal.id} className="island-control-row" disabled={data.loading} onClick={() => model.action({ kind: 'openAgentTerminal', ref: data.ref, id: terminal.id })}><span>{terminal.backend || 'Node.js'} · {tr(terminal.step)}</span><span>{terminal.exitCode === null ? tr('Running') : `${tr('Exit code')}: ${terminal.exitCode}`}</span></button>)}
      </IntegrationSection>
      <IntegrationSection data={data} section="memories" title="Unsure memories" empty="No unsure memories reported" hasItems={!!data.memories.length}>
        {data.memories.map((memory) => <button key={`${memory.shared}/${memory.file}`} className="island-control-row vertical" disabled={data.loading} onClick={() => model.action({ kind: 'openMemory', ref: data.ref })}><span>{memory.description || memory.file}</span><span className="island-subtitle">{memory.confirmed ? `${tr('Confirmed')}: ${memory.confirmed}` : tr('Never confirmed')}</span></button>)}
      </IntegrationSection>
      <IntegrationSection data={data} section="artifacts" title="Artifact revisions" empty="No revised artifacts reported" hasItems={data.artifacts.some((artifact) => artifact.revised)}>
        {data.artifacts.filter((artifact) => artifact.revised).map((artifact) => <button key={artifact.id} className="island-control-row vertical" disabled={data.loading} onClick={() => model.action({ kind: 'openIntegrationArtifact', ref: data.ref, id: artifact.id })}><span>{artifact.title}</span><span className="island-subtitle">{tr('Open latest revision')} · {new Date(artifact.updatedAt).toLocaleString()}</span></button>)}
      </IntegrationSection>
      <IntegrationSection data={data} section="delegations" title="Waiting for another bot" empty="No waiting bot requests reported" hasItems={!!data.delegations.length}>
        {data.delegations.map((request) => <button key={request.id} className="island-control-row vertical" onClick={() => model.openBot(`${computer}/${request.to}`)}><span>{name(request.from)} → {name(request.to)}</span><time className="island-subtitle">{new Date(request.at).toLocaleString()}</time></button>)}
      </IntegrationSection>
    </> : null}
    <section className="island-integration-section" aria-label={tr('Approval age')}><strong>{tr('Approval age')}</strong>
      {approvals.length ? approvals.map((approval) => { const age = islandApprovalAge(approval.createdAt, Date.now()); return <button key={approval.entryId} className="island-control-row" onClick={() => model.setTab('approvals')}><span>{approval.bot.name} · {approval.headline}</span><span>{age === null ? tr('Not reported') : `${age} ${tr('minutes')}`}</span></button> }) : <span className="island-subtitle">{tr('No pending approvals')}</span>}
    </section>
    <section className="island-integration-section" aria-label={tr('File uploads')}><strong>{tr('File uploads')}</strong>
      {uploads.length ? uploads.map((upload, index) => <div key={`${upload.ref}/${upload.name}/${index}`} className="island-control-row"><span>{upload.name}</span><span>{upload.percent === null ? tr(upload.status) : `${upload.percent}% · ${tr(upload.status)}`}</span></div>) : <span className="island-subtitle">{tr('No active uploads reported')}</span>}
    </section>
    <section className="island-integration-section" aria-label={tr('Dictation')}><strong>{tr('Dictation')}</strong>
      {signals?.dictation ? <div className="island-control-row"><span>{tr('Main window')} · {tr(signals.dictation.phase)}</span><Button kind="secondary" onClick={() => signals.dictation && model.action({ kind: 'stopDictation', id: signals.dictation.id })}>{tr('Stop dictation')}</Button></div> : null}
      {model.dictation.busy ? <div className="island-control-row"><span>{tr('Island')} · {tr(model.dictation.transcribing ? 'Transcribing' : 'Listening')}</span><Button kind="secondary" onClick={() => model.dictation.cancel()}>{tr('Stop dictation')}</Button></div> : null}
      {!signals?.dictation && !model.dictation.busy ? <span className="island-subtitle">{tr('No active dictation')}</span> : null}
    </section>
  </>
}

function ToolIntegrations({ model, data }: { model: IslandModel; data: IslandIntegrations | null }) {
  const tr = useTr()
  const signals = model.summary.signals
  return <>
    <section className="island-integration-section" aria-label={tr('Computer connections')}><strong>{tr('Computer connections')}</strong>
      {signals?.computers.length ? signals.computers.map((computer) => { const bot = model.summary.bots.find((bot) => bot.ref.split('/')[0] === computer.id); return <div className="island-integration-item" key={computer.id}><div className="island-control-row"><span>{computer.name} · {tr(computer.state)}</span>{computer.state !== 'online' && bot ? <Button kind="secondary" disabled={data?.busy} onClick={() => { model.selectFocus(bot.ref); model.action({ kind: 'reconnectComputer', ref: bot.ref }) }}>{tr('Reconnect')}</Button> : null}</div>{computer.error ? <span className="island-control-error">{computer.error}</span> : null}</div> }) : <span className="island-subtitle">{tr('Connection status unavailable')}</span>}
    </section>
    {data ? <>
      <IntegrationSection data={data} section="connectors" title="Connector sign-in" empty="No connectors need attention" hasItems={!!data.connectors.length}>
        {data.connectors.map((connector) => <button key={connector.id} className="island-control-row vertical" disabled={data.loading} onClick={() => model.action({ kind: 'openConnector', ref: data.ref, id: connector.id })}><span>{connector.name} · {tr(connector.auth)}</span><span className="island-subtitle">{connector.failure || tr('Open sign-in controls')}</span></button>)}
      </IntegrationSection>
      <IntegrationSection data={data} section="searches" title="Saved searches" empty="No saved searches" hasItems={!!data.searches.length}>
        {data.searches.map((search) => <button key={search.id} className="island-control-row" disabled={data.loading || data.search?.loading} onClick={() => model.action({ kind: 'runSavedSearch', ref: data.ref, id: search.id })}><span>{search.query}</span><span className="island-subtitle">{tr(search.scope)}</span></button>)}
      </IntegrationSection>
      {data.search ? <section className="island-integration-section" aria-label={tr('Search results')}><strong>{tr('Search results')} · {tr('This bot')}</strong>
        {data.search.loading ? <span className="island-subtitle" role="status">{tr('Searching…')}</span> : data.search.error ? <span className="island-control-error" role="status">{data.search.error}</span> : data.search.hits.length ? data.search.hits.map((hit) => <button className="island-control-row vertical" key={hit.id} onClick={() => model.action({ kind: 'openSearchHit', ref: data.ref, id: hit.id })}><span>{hit.text}</span><span className="island-subtitle">{model.summary.bots.find((bot) => bot.ref === `${data.ref.split('/')[0]}/${hit.botId}`)?.name ?? hit.botId}</span></button>) : <span className="island-subtitle">{tr('No search results')}</span>}
      </section> : null}
      <IntegrationSection data={data} section="bookmarks" title="Bookmarks" empty="No bookmarks" hasItems={!!data.bookmarks.length}>
        {data.bookmarks.map((bookmark) => <button className="island-control-row" key={bookmark.id} disabled={data.loading} onClick={() => model.action({ kind: 'openBookmark', ref: data.ref, id: bookmark.id })}><span>{bookmark.title || tr('Saved message')}</span><Icon name="arrow.up.right.square" size={12} /></button>)}
      </IntegrationSection>
    </> : null}
  </>
}
