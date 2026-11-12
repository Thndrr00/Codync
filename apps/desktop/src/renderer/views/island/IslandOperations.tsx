import { useEffect, useState, type ReactNode } from 'react'
import { islandAppFavoriteKey, islandContextPercent, type IslandOperations } from '@shared/island-operations'
import { bytesText } from '@shared/health'
import { Button, ChipChoice } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { pref, usePref } from '../../lib/prefs'
import { useTr } from '../../lib/use-language'
import type { IslandModel } from './island-model'
import './island-operations.css'

type OperationsTab = 'runtime' | 'project' | 'system'
const favoriteApps = pref<string[]>('islandFavoriteApps', [])
const count = (value: number | null) => value !== null && Number.isFinite(value) && value >= 0 ? new Intl.NumberFormat().format(value) : null
const dateText = (value: number | null) => value !== null && Number.isFinite(value) && value > 0 && Number.isFinite(new Date(value).getTime()) ? new Date(value).toLocaleString() : null

/** Operational projections and existing destinations; this view never reads the host directly. */
export function OperationsView({ model }: { model: IslandModel }) {
  const tr = useTr()
  const [tab, setTab] = useState<OperationsTab>('runtime')
  const ref = model.focus?.ref
  const data = model.summary.operations?.ref === ref ? model.summary.operations : null
  const [rawFavorites, saveFavorites] = usePref(favoriteApps)
  const favorites = Array.isArray(rawFavorites) ? rawFavorites.filter((item): item is string => typeof item === 'string').slice(0, 50) : []
  const toggleFavorite = (key: string) => saveFavorites(favorites.includes(key) ? favorites.filter((item) => item !== key) : [key, ...favorites].slice(0, 50))
  useEffect(() => { if (ref) model.action({ kind: 'loadOperations', ref }) }, [model, ref])
  return <div className="island-operations">
    <div className="island-control-heading"><strong>{tr('Operations')}</strong><button className="island-tab" title={tr('Refresh operations')} aria-label={tr('Refresh operations')} disabled={!ref || data?.loading} onClick={() => ref && model.action({ kind: 'loadOperations', ref })}><Icon name="arrow.clockwise" size={13} /></button></div>
    <ChipChoice<OperationsTab> label="Operation section" selection={tab} options={[{ id: 'runtime', label: 'Runtime' }, { id: 'project', label: 'Project' }, { id: 'system', label: 'System' }]} onChange={setTab} />
    {ref && !data ? <span className="island-subtitle" role="status">{tr('Loading operations…')}</span> : null}
    {data?.loading ? <span className="island-subtitle" role="status">{tr('Refreshing operations…')}</span> : null}
    {data?.at ? <time className="island-subtitle">{tr('Updated')} · {dateText(data.at) ?? tr('Not reported')}</time> : null}
    {data?.errors.find((error) => error.section === 'connection') ? <span className="island-control-error" role="status">{data.errors.find((error) => error.section === 'connection')!.message}</span> : null}
    {data?.actionError ? <span className="island-control-error" role="alert">{data.actionError}</span> : null}
    {tab === 'runtime' && data ? <RuntimeOperations model={model} data={data} /> : null}
    {tab === 'project' && data ? <ProjectOperations model={model} data={data} favorites={favorites} toggleFavorite={toggleFavorite} /> : null}
    {tab === 'system' ? <SystemOperations model={model} data={data} /> : null}
  </div>
}

function OperationSection({ data, section, title, hasItems, empty, children }: { data: IslandOperations; section: string; title: string; hasItems: boolean; empty: string; children: ReactNode }) {
  const tr = useTr()
  const error = data.errors.find((error) => error.section === section)
  const disconnected = data.errors.some((error) => error.section === 'connection')
  return <section className="island-operation-section" aria-label={tr(title)}>
    <strong>{tr(title)}</strong>
    {error ? <span className="island-control-error" role="status">{tr('Unavailable')} · {error.message}</span> : null}
    {hasItems ? children : disconnected ? <span className="island-subtitle">{tr('Unavailable')}</span> : !error ? <span className="island-subtitle">{tr(data.loading ? 'Loading…' : empty)}</span> : null}
  </section>
}

function RuntimeOperations({ model, data }: { model: IslandModel; data: IslandOperations }) {
  const tr = useTr()
  const runtime = data.runtime
  const context = runtime?.context ?? null
  const percent = islandContextPercent(context)
  const usage = data.usage
  const health = data.health
  return <>
    <OperationSection data={data} section="runtime" title="Context and compaction" hasItems={!!runtime || !!data.compactions.length} empty="Context usage unavailable">
      {runtime?.error ? <span className="island-control-error">{runtime.error}</span> : null}
      {context ? <>
        <div className="island-control-row"><span>{tr('Last reported main-session context')}</span><span>{count(context.used) ?? tr('Not reported')}{context.size !== null ? ` / ${count(context.size) ?? tr('Not reported')}` : ''}{percent !== null ? ` · ${percent}%` : ''}</span></div>
        <span className="island-subtitle">{tr('Last completed-turn report')}: {dateText(context.lastTurn) ?? tr('Not reported')}</span>
        <div className="island-control-row"><span>{tr('Reported compactions')}</span><span>{count(context.compactions) ?? tr('Not reported')}</span></div>
      </> : <span className="island-subtitle">{tr('Context usage unavailable')}</span>}
      {data.compactions.map((compaction) => <div className="island-control-row" key={compaction.id}><span>{tr(compaction.byHost ? 'Host compaction' : 'Agent compaction')}</span><time>{dateText(compaction.at) ?? tr('Not reported')}</time></div>)}
    </OperationSection>
    <OperationSection data={data} section="usage" title="Tokens and cost" hasItems={!!usage || !!runtime} empty="Token usage unavailable">
      {usage ? <>
        <div className="island-control-row"><span>{tr('Tokens today · UTC')}</span><span>{count(usage.used) ?? tr('Not reported')}{usage.limit !== null ? ` / ${count(usage.limit) ?? tr('Not reported')}` : ''}</span></div>
        <span className="island-subtitle">{tr('Reset')}: {dateText(usage.resetsAt) ?? tr('Not reported')}</span>
        <div className="island-control-row"><span>{tr('All bots on this computer')}</span><span>{count(usage.fleetUsed) ?? tr('Not reported')}{usage.fleetLimit !== null ? ` / ${count(usage.fleetLimit) ?? tr('Not reported')}` : ''}</span></div>
      </> : <span className="island-subtitle">{tr('Token usage unavailable')}</span>}
      <span className="island-subtitle">{tr('Cost not reported by the host.')}</span>
    </OperationSection>
    <OperationSection data={data} section="budgets" title="Token budgets" hasItems={!!data.budgets.length} empty="No token limits reported">
      {data.budgets.map((budget) => <div key={budget.botId} className="island-control-row"><span>{budget.botId === '*' ? tr('All bots') : model.summary.bots.find((bot) => bot.ref === `${data.ref.split('/')[0]}/${budget.botId}`)?.name ?? tr('This bot')} · {tr(budget.level)}</span><span>{count(budget.used)} / {count(budget.limit)}</span></div>)}
    </OperationSection>
    <Button onClick={() => model.action({ kind: 'openOperationPanel', ref: data.ref, panel: 'budget' })}>{tr('Open budget settings')}</Button>
    <OperationSection data={data} section="fallback" title="Agent fallback" hasItems={!!data.fallback || !!data.switches.length} empty="No fallback reported">
      {data.fallback ? <><span>{data.fallback.agent}{data.fallback.model ? ` · ${data.fallback.model}` : ''}</span><span className="island-subtitle">{data.fallback.reason ? tr(data.fallback.reason) : tr('Reason not reported')} · {dateText(data.fallback.since) ?? tr('Not reported')}</span></> : null}
    </OperationSection>
    {data.switches.length ? <><strong>{tr('Loaded agent changes')}</strong>{data.switches.map((change) => <div className="island-control-row vertical" key={change.id}><span>{change.from} → {change.to}{change.back ? ` · ${tr('Returned to own agent')}` : ''}</span><span className="island-subtitle">{change.reason ? tr(change.reason) : tr('Reason not reported')} · {dateText(change.at) ?? tr('Not reported')}</span></div>)}</> : null}
    <OperationSection data={data} section="health" title="Computer resources" hasItems={!!health} empty="Resource readings unavailable">
      {health ? <><div className="island-control-row"><span>CPU</span><span>{Number.isFinite(health.cpu) ? `${Math.round(health.cpu)}%` : tr('Not reported')}</span></div><div className="island-control-row"><span>{tr('Memory')}</span><span>{Number.isFinite(health.ramUsedPercent) ? `${health.ramUsedPercent}%` : tr('Not reported')} · {bytesText(health.ramFree)} / {bytesText(health.ramTotal)} {tr('free')}</span></div><div className="island-control-row"><span>{tr('Disk free')}</span><span>{health.diskTotal > 0 ? `${bytesText(health.diskFree)} / ${bytesText(health.diskTotal)}` : tr('Not reported')}</span></div><div className="island-control-row"><span>{tr('Selected agent memory')}</span><span>{health.rss !== null ? bytesText(health.rss) : tr('Not reported')}</span></div><time className="island-subtitle">{tr('Sampled')}: {dateText(health.at) ?? tr('Not reported')}</time></> : null}
    </OperationSection>
    <OperationSection data={data} section="runtime" title="MCP requests" hasItems={!!runtime?.mcp.length} empty="No current MCP requests reported">
      {runtime?.mcp.map((request) => <button className="island-control-row vertical" key={request.id} onClick={() => model.action({ kind: 'openOperationPanel', ref: data.ref, panel: 'trace' })}><span>{request.tool} · {tr(request.status)}</span><span className="island-subtitle">{request.connectorId} · {dateText(request.createdAt) ?? tr('Not reported')}</span>{request.status === 'dispatched' ? <span className="island-subtitle">{tr('Dispatched; completion has not been reported.')}</span> : null}</button>)}
    </OperationSection>
    <Button onClick={() => model.action({ kind: 'openOperationPanel', ref: data.ref, panel: 'trace' })}>{tr('Open trace')}</Button>
  </>
}

function ProjectOperations({ model, data, favorites, toggleFavorite }: { model: IslandModel; data: IslandOperations; favorites: string[]; toggleFavorite: (key: string) => void }) {
  const tr = useTr()
  const apps = [...data.apps].sort((a, b) => Number(favorites.includes(islandAppFavoriteKey(data.ref, b.slug))) - Number(favorites.includes(islandAppFavoriteKey(data.ref, a.slug))))
  return <>
    <OperationSection data={data} section="resources" title="Recent documents" hasItems={!!data.resources?.documents.length} empty="No documents reported">
      {data.resources?.documents.map((document) => <button className="island-control-row" key={document.id} disabled={data.loading} onClick={() => model.action({ kind: 'openOperationDocument', ref: data.ref, id: document.id })}><span>{document.title}</span><time>{dateText(document.updatedAt) ?? tr('Not reported')}</time></button>)}
    </OperationSection>
    {data.resources?.error ? <span className="island-control-error">{data.resources.error}</span> : null}
    <OperationSection data={data} section="apps" title="Mini-app favorites" hasItems={!!apps.length} empty="No mini-apps reported">
      {apps.map((app) => { const key = islandAppFavoriteKey(data.ref, app.slug), pinned = favorites.includes(key); return <div className="island-control-row" key={app.slug}><button className="island-control-row" disabled={data.loading} onClick={() => model.action({ kind: 'openOperationApp', ref: data.ref, id: app.slug })}><span>{app.title}</span><span className="island-subtitle">v{app.version}</span></button><button className="island-pin" aria-pressed={pinned} aria-label={tr(pinned ? 'Unpin mini-app' : 'Pin mini-app')} title={tr(pinned ? 'Unpin mini-app' : 'Pin mini-app')} onClick={() => toggleFavorite(key)}><Icon name={pinned ? 'pin.fill' : 'pin'} size={12} /></button></div> })}
    </OperationSection>
    <OperationSection data={data} section="git" title="Git conflicts" hasItems={!!data.git?.conflicts.length} empty={data.git ? 'No conflict files reported' : 'Git status unavailable'}>
      {data.git?.conflicts.map((path) => <button className="island-control-row" key={path} onClick={() => model.openBot(data.ref)}><span>{path}</span><Icon name="arrow.up.right.square" size={12} /></button>)}
    </OperationSection>
    {data.git?.error ? <span className="island-control-error">{data.git.error}</span> : null}
    <OperationSection data={data} section="git" title="Project worktrees" hasItems={!!data.git?.worktrees.length} empty={data.git ? 'No worktrees reported' : 'Git status unavailable'}>
      <span className="island-subtitle">{tr('Current worktree')}: {data.git?.root ?? tr('Not reported')}</span>
      {data.git?.worktrees.map((worktree) => <div className="island-operation-item" key={worktree.path}><span>{worktree.path}</span><span className="island-subtitle">{worktree.branch ?? tr(worktree.detached ? 'Detached HEAD' : 'Branch not reported')}{worktree.locked ? ` · ${tr('Locked')}` : ''}{worktree.prunable ? ` · ${tr('Prunable')}` : ''}</span>{worktree.botIds.length ? <div className="island-control-actions">{worktree.botIds.map((id) => <Button key={id} onClick={() => model.action({ kind: 'selectWorktreeBot', ref: data.ref, id })}>{model.summary.bots.find((bot) => bot.ref === `${data.ref.split('/')[0]}/${id}`)?.name ?? tr('Open bot')}</Button>)}</div> : <span className="island-subtitle">{tr('No existing bot is assigned to this worktree.')}</span>}</div>)}
    </OperationSection>
    <OperationSection data={data} section="computer" title="Computer-use session" hasItems={!!data.computer} empty="Computer-use state unavailable">
      {data.computer ? <><span>{tr(data.computer.enabled ? 'Computer enabled' : 'Computer disabled')}{data.computer.enabled ? ` · ${tr(data.computer.userControl ? 'User control' : data.computer.agentBot ? 'Agent control' : 'No active agent')}` : ''}</span><span className="island-subtitle">{tr('Selected bot computer access')}: {tr(data.computer.computerUse ? 'Enabled' : 'Disabled')}</span><span className="island-subtitle">{tr('Active agent')}: {data.computer.agentBot ?? tr('Not reported')}</span></> : null}
    </OperationSection>
    <div className="island-control-actions"><Button onClick={() => model.action({ kind: 'openOperationPanel', ref: data.ref, panel: 'computer' })}>{tr('Open computer session')}</Button><Button disabled={!data.localComputer} onClick={() => model.action({ kind: 'openExistingBrowser', ref: data.ref })}>{tr('Open existing browser')}</Button></div>
    {data.localComputer ? <span className="island-subtitle">{tr('Desktop browser window')}: {tr(data.browserOpen === null ? 'Not reported' : data.browserOpen ? 'Open' : 'Closed')}</span> : null}
    {data.errors.find((error) => error.section === 'browser') ? <span className="island-control-error">{data.errors.find((error) => error.section === 'browser')!.message}</span> : null}
    {!data.localComputer ? <span className="island-subtitle">{tr('Open browser windows on their computer.')}</span> : null}
  </>
}

function SystemOperations({ model, data }: { model: IslandModel; data: IslandOperations | null }) {
  const tr = useTr()
  const signals = model.summary.operationSignals
  const updates = signals?.updates
  return <>
    {signals?.updateError ? <span className="island-control-error" role="status">{signals.updateError}</span> : null}
    <section className="island-operation-section" aria-label={tr('App updates')}><strong>{tr('App updates')}</strong>
      {updates?.supported ? <><span>{tr(updates.installing ? 'Installing' : updates.staged ? 'Ready to install' : updates.checking ? 'Checking for updates' : updates.availableVersion ? 'Update available' : 'No update reported')}{updates.availableVersion ? ` · ${updates.availableVersion}` : ''}</span>{updates.downloadPercent !== null && Number.isFinite(updates.downloadPercent) ? <span>{tr('Download')}: {Math.round(updates.downloadPercent)}%</span> : null}{updates.error ? <span className="island-control-error">{updates.error}</span> : null}</> : <span className="island-subtitle">{tr('App update state unavailable')}</span>}
      <Button onClick={() => model.action({ kind: 'openOperationSettings', page: 'updates' })}>{tr('Open update settings')}</Button>
    </section>
    {data ? <OperationSection data={data} section="hostUpdate" title="Host updates" hasItems={!!data.hostUpdate} empty="Host update state unavailable">{data.hostUpdate ? <><span>{data.hostUpdate.currentVersion} · {tr(data.hostUpdate.phase)}{data.hostUpdate.availableVersion ? ` · ${data.hostUpdate.availableVersion}` : ''}</span>{data.hostUpdate.error ? <span className="island-control-error">{data.hostUpdate.error}</span> : null}</> : null}</OperationSection> : null}
    <section className="island-operation-section" aria-label={tr('SSH tunnels')}><strong>{tr('SSH tunnels')}</strong>
      {signals ? signals.ssh.length ? signals.ssh.map((tunnel) => <div className="island-operation-item" key={tunnel.id}><span>{tunnel.name} · {tr(tunnel.kind)}</span>{tunnel.detail ? <span className="island-subtitle">{tunnel.detail}</span> : null}{tunnel.retryable ? <Button kind="secondary" onClick={() => model.action({ kind: 'retryIslandSSH', id: tunnel.id })}>{tr('Retry connection')}</Button> : null}</div>) : <span className="island-subtitle">{tr('No SSH tunnels reported')}</span> : <span className="island-subtitle">{tr('SSH tunnel state unavailable')}</span>}
      <Button onClick={() => model.action({ kind: 'openOperationSettings', page: 'computers' })}>{tr('Open computers')}</Button>
    </section>
    {data ? <OperationSection data={data} section="resources" title="Channel diagnostics · this computer" hasItems={!!data.resources} empty="Channel state unavailable">
      {data.resources ? <>{data.resources.channelState.map((channel) => <div className="island-control-row" key={channel.source}><span>{channel.source}</span><span>{tr(channel.state)}</span></div>)}{data.resources.channelFailures.map((failure) => <div className="island-control-row" key={`${failure.source}/${failure.id}`}><span>{failure.source}</span><span className="island-control-error">{tr(failure.state)}</span></div>)}<span className="island-subtitle">{tr('Telegram delivery status is not reported.')}</span></> : null}
      {data.localComputer ? <Button onClick={() => model.action({ kind: 'openOperationSettings', page: 'channels' })}>{tr('Open channel diagnostics')}</Button> : <span className="island-subtitle">{tr('Configure channels on their computer.')}</span>}
    </OperationSection> : null}
    <section className="island-operation-section" aria-label={tr('Device access requests')}><strong>{tr('Device access requests')}</strong>
      {signals ? signals.devices.length ? signals.devices.map((device) => <div className="island-control-row vertical" key={`${device.computer}/${device.kind}/${device.id}`}><span>{device.name} · {tr(device.kind)}</span><span className="island-subtitle">{device.computer}{device.platform ? ` · ${device.platform}` : ''}</span></div>) : <span className="island-subtitle">{tr('No device requests reported')}</span> : <span className="island-subtitle">{tr('Device request state unavailable')}</span>}
      <Button onClick={() => model.action({ kind: 'reviewDevices' })}>{tr('Review device requests')}</Button>
    </section>
  </>
}
