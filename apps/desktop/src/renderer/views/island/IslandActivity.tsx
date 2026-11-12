import { useEffect, useState } from 'react'
import type { IslandRun } from '@shared/island-activity'
import { elapsedIslandRun, inIslandScope, type IslandScope } from '@shared/island-workspace'
import { Button, ChipChoice } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { useTr } from '../../lib/use-language'
import type { IslandModel } from './island-model'
import { IslandPanel } from './IslandPanel'

/** Scope presentation only; routine execution and retry eligibility remain host-owned. */
export function ActivityView({ model }: { model: IslandModel }) {
  const tr = useTr()
  const [scope, setScope] = useState<IslandScope>('all')
  const activity = model.summary.activity
  const selected = model.focus
  useEffect(() => { if (selected) model.action({ kind: 'loadWorkspace', ref: selected.ref }) }, [model, selected?.ref])
  const includes = (ref: string) => inIslandScope(ref, scope, selected, model.summary.bots)
  const unread = (activity?.unread ?? model.summary.bots.filter((bot) => (bot.unread ?? 0) > 0).map((bot) => ({ ref: bot.ref, name: bot.name, count: bot.unread ?? 0 }))).filter((bot) => includes(bot.ref))
  const runs = activity?.runs.filter((run) => includes(run.ref)) ?? []
  const channels = activity?.channels.filter((event) => includes(event.ref)) ?? []
  const approvals = model.summary.permissions.filter((card) => includes(card.ref)).length
  return <IslandPanel>
    <div className="island-control-heading"><strong>{tr('Activity')}</strong><button className="island-tab" title={tr('Refresh')} aria-label={tr('Refresh')} onClick={() => model.action({ kind: 'refreshActivity' })}><Icon name="arrow.clockwise" size={13} /></button></div>
    <ChipChoice<IslandScope> label="Activity scope" selection={scope} options={[{ id: 'all', label: 'All' }, { id: 'bot', label: 'Bot' }, { id: 'project', label: 'Project' }]} onChange={setScope} />
    {scope !== 'all' && model.summary.bots.length ? <ChipChoice label="Select bot" selection={selected?.ref ?? ''} options={model.summary.bots.map((bot) => ({ id: bot.ref, label: `${bot.name} · ${bot.folder}` }))} onChange={(ref) => model.selectFocus(ref)} /> : null}
    {scope === 'project' && !selected?.project ? <span className="island-subtitle">{tr('Select a bot with a project to use this scope.')}</span> : null}
    <div className="island-control-actions"><Button onClick={() => model.setTab('approvals')}>{tr('Approvals')} · {approvals}</Button><Button onClick={() => model.action({ kind: 'openChannels' })}>{tr('Channels')}</Button></div>
    <strong>{tr('Unread conversations')}</strong>
    {unread.length ? unread.map((bot) => <button className="island-control-row" key={bot.ref} onClick={() => model.openBot(bot.ref)}><span>{bot.name}</span><span>{bot.count}</span></button>) : <span className="island-subtitle">{tr('No unread messages')}</span>}
    <strong>{tr('Routines')}</strong>
    {runs.length ? runs.map((run) => <RunRow key={run.id} model={model} run={run} />) : <span className="island-subtitle">{tr('No recent runs')}</span>}
    <strong>{tr('Channel mentions')}</strong>
    {channels.length ? channels.map((event) => <button className="island-control-row" key={event.id} onClick={() => model.openBot(event.ref)}><span>{event.source} · {event.channel}</span><time>{new Date(event.at).toLocaleTimeString()}</time></button>) : <span className="island-subtitle">{tr('No recent mentions')}</span>}
    {activity?.error ? <span className="island-control-error" role="status">{activity.error}</span> : null}
    {model.summary.workspace?.actionError ? <span className="island-control-error" role="alert">{model.summary.workspace.actionError}</span> : null}
  </IslandPanel>
}

function RunRow({ model, run }: { model: IslandModel; run: IslandRun }) {
  const tr = useTr()
  const workspace = model.summary.workspace?.ref === run.ref ? model.summary.workspace : null
  const estimate = workspace?.timings.find((timing) => timing.routineId === run.routineId && timing.samples >= 5 && Number.isFinite(timing.estimateMs) && timing.estimateMs > 0)
  const finished = run.finishedAt !== null && run.finishedAt !== undefined
  const elapsed = finished || run.status === 'running' ? elapsedIslandRun(run.startedAt, run.finishedAt, Date.now()) : null
  return <div className="island-control-row vertical">
    <button className="island-control-row" onClick={() => model.action({ kind: 'openFlow', ref: run.ref })}><span>{run.name} · {tr(run.status)}</span><Icon name="arrow.up.right.square" size={12} /></button>
    <span className="island-subtitle">{run.detail || new Date(run.at).toLocaleString()}</span>
    <span className="island-subtitle">{tr('Elapsed')}: {elapsed === null ? tr('Not reported') : `${elapsed} ${tr('seconds')}`}</span>
    <span className="island-subtitle">{estimate ? `${tr('Historical estimate')}: ${Math.round(estimate.estimateMs / 1000)} ${tr('seconds')} · ${estimate.samples} ${tr('samples')}` : tr('No estimate')}</span>
    {run.steps.map((step, index) => <span key={index} className="island-subtitle">{step.name} · {tr(step.status)}</span>)}
    {run.retryable ? <Button kind="secondary" disabled={workspace?.busy || workspace?.loading || model.summary.activity?.online === false} onClick={() => model.action({ kind: 'retryRun', ref: run.ref, id: run.id })}>{tr('Retry run')}</Button> : null}
  </div>
}
