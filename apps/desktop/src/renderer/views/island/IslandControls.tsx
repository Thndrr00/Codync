import type { ReactNode } from 'react'
import type { IslandEdge } from '@shared/island-layout'
import { Button, Switch } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { useTr } from '../../lib/use-language'
import type { IslandModel } from './island-model'
import { useIslandUsage } from './island-preferences'

export function IslandPanel({ children }: { children: ReactNode }) {
  return <div className="island-card island-control-panel" tabIndex={0}>{children}</div>
}

function Choices({ label, value, options, onChange }: { label: string; value: string; options: { id: string; label: string }[]; onChange: (id: string) => void }) {
  return <div role="group" aria-label={label}><span className="island-subtitle">{label}</span><div className="island-control-actions">{options.map((option) => <Button key={option.id} kind={value === option.id ? 'primary' : 'secondary'} aria-pressed={value === option.id} onClick={() => onChange(option.id)}>{option.label}</Button>)}</div></div>
}

export function ActivityView({ model }: { model: IslandModel }) {
  const tr = useTr()
  const activity = model.summary.activity
  const unread = activity?.unread ?? model.summary.bots.filter((bot) => (bot.unread ?? 0) > 0).map((bot) => ({ ...bot, count: bot.unread }))
  return (
    <IslandPanel>
      <div className="island-control-heading"><strong>{tr('Activity')}</strong><button className="island-tab" title={tr('Refresh')} aria-label={tr('Refresh')} onClick={() => model.action({ kind: 'refreshActivity' })}><Icon name="arrow.clockwise" size={13} /></button></div>
      <div className="island-control-actions"><Button onClick={() => model.setTab('approvals')}>{tr('Approvals')} · {model.summary.permissions.length}</Button><Button onClick={() => model.action({ kind: 'openChannels' })}>{tr('Channels')}</Button></div>
      <strong>{tr('Unread conversations')}</strong>
      {unread.length ? unread.map((bot) => <button className="island-control-row" key={bot.ref} onClick={() => model.openBot(bot.ref)}><span>{bot.name}</span><span>{bot.count}</span></button>) : <span className="island-subtitle">{tr('No unread messages')}</span>}
      <strong>{tr('Routines')}</strong>
      {activity?.runs.length ? activity.runs.map((run) => <button className="island-control-row vertical" key={run.id} onClick={() => model.action({ kind: 'openFlow', ref: run.ref })}>
        <span>{run.name} · {tr(run.status)}</span><span className="island-subtitle">{run.detail || new Date(run.at).toLocaleString()}</span>
        {run.steps.map((step, index) => <span key={index} className="island-subtitle">{step.name} · {tr(step.status)}</span>)}
      </button>) : <span className="island-subtitle">{tr('No recent runs')}</span>}
      <strong>{tr('Channel mentions')}</strong>
      {activity?.channels.length ? activity.channels.map((event) => <button className="island-control-row" key={event.id} onClick={() => model.openBot(event.ref)}><span>{event.source} · {event.channel}</span><time dateTime={new Date(event.at).toISOString()}>{new Date(event.at).toLocaleTimeString()}</time></button>) : <span className="island-subtitle">{tr('No recent mentions')}</span>}
      {activity?.error ? <span className="island-control-error" role="status">{activity.error}</span> : null}
    </IslandPanel>
  )
}

export function StatusView({ model }: { model: IslandModel }) {
  const tr = useTr()
  const activity = model.summary.activity
  const call = activity?.call
  return <IslandPanel>
    <strong>{tr('Connection and health')}</strong>
    <span>{tr(activity?.connection ?? 'Connecting…')} · {tr(activity?.route ?? 'Unknown')}</span>
    {activity?.health.map((line) => <div className="island-control-row" key={line.label}><span>{tr(line.label)}</span><span>{line.value}</span></div>)}
    <Button onClick={() => model.action({ kind: 'openDiagnostics' })}>{tr('Diagnostics')}</Button>
    <strong>{tr('Voice call')}</strong>
    {call ? <><span>{call.name} · {tr(call.phase)}</span><div className="island-control-actions"><Button onClick={() => model.action({ kind: 'callMute', callId: call.id })}>{tr(call.muted ? 'Unmute' : 'Mute')}</Button><Button kind="secondary" className="island-danger" onClick={() => model.action({ kind: 'callEnd', callId: call.id })}>{tr('End call')}</Button><Button onClick={() => model.openBot(call.ref)}>{tr('Open conversation')}</Button></div></> : <span className="island-subtitle">{tr('No active call')}</span>}
    <strong>n8n</strong>
    <span>{tr(activity?.n8n?.state ?? 'Not configured')}{activity?.n8n?.mcp ? ` · ${tr('MCP connected')}` : ''}</span>
    {activity?.n8n?.error ? <span className="island-control-error">{activity.n8n.error}</span> : null}
    <Button onClick={() => void model.openN8n()}>{tr('Open n8n')}</Button>
    {model.actionError ? <span className="island-control-error" role="status">{model.actionError}</span> : null}
  </IslandPanel>
}

export function OptionsView({ model }: { model: IslandModel }) {
  const tr = useTr()
  const controls = model.controls
  const [usage, setUsage] = useIslandUsage()
  return <IslandPanel>
    <strong>{tr('Island options')}</strong>
    <div className="island-control-actions" role="group" aria-label={tr('Screen edge')}>
      {(['top', 'bottom', 'left', 'right'] as IslandEdge[]).map((edge) => <button className={`island-option ${model.screen.edge === edge ? 'selected' : ''}`} key={edge} aria-pressed={model.screen.edge === edge} onClick={() => model.setEdge(edge)}>{tr({ top: 'Top', bottom: 'Bottom', left: 'Left', right: 'Right' }[edge])}</button>)}
    </div>
    <Choices label={tr('Display')} value={controls.displayId === null ? controls.placement : String(controls.displayId)} options={[
      { id: 'automatic', label: tr('Automatic') }, { id: 'primary', label: tr('Main display') }, { id: 'follow', label: tr('Follow pointer') },
      ...(model.screen.displays ?? []).map((display) => ({ id: String(display.id), label: display.label })),
    ]} onChange={(value) => void model.configure(['automatic', 'primary', 'follow'].includes(value) ? { placement: value as typeof controls.placement, displayId: null } : { displayId: Number(value) })} />
    {controls.displayId !== null && !model.screen.displays?.some((display) => display.id === controls.displayId) ? <span className="island-subtitle">{tr('Saved display is unavailable; using an available display.')}</span> : null}
    <Choices label={tr('Density')} value={controls.density} options={[{ id: 'compact', label: tr('Compact') }, { id: 'expanded', label: tr('Expanded') }]} onChange={(value) => void model.configure({ density: value === 'expanded' ? 'expanded' : 'compact' })} />
    <Switch label="Lock position" on={controls.locked} onChange={(locked) => void model.configure({ locked })}>Lock position</Switch>
    <span className="island-subtitle">{tr('Unlock, then drag the handle to a screen edge. Positions are saved per display.')}</span>
    <Switch label="Usage notices" on={usage.notices} onChange={(notices) => setUsage({ ...usage, notices })}>Usage notices</Switch>
    <Choices label={tr('Usage warning')} value={String(usage.threshold)} options={[70, 80, 90, 95, 100].map((value) => ({ id: String(value), label: `${value}%` }))} onChange={(value) => setUsage({ ...usage, threshold: Number(value) })} />
    <span className="island-subtitle">{tr('Notices stay in the island; resets within 15 minutes are highlighted.')}</span>
    {model.configurationError ? <span className="island-control-error" role="alert">{model.configurationError}</span> : null}
  </IslandPanel>
}
