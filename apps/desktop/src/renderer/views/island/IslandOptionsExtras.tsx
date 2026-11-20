import { useEffect, useState } from 'react'
import { defaultIslandBindings, islandShortcutActions, type IslandShortcutAction } from '@shared/island-shortcuts'
import { Button, SegmentedChoice, Switch } from '../../components/Controls'
import { useTr } from '../../lib/use-language'
import { AGENT_DOCK_SIZE, useAgentDockPreference, type AgentDockEdge } from './agent-dock-preference'
import type { IslandModel } from './island-model'

const labels: Record<IslandShortcutAction, string> = {
  toggle: 'Show or hide island', search: 'Workspace and search', activity: 'Activity', message: 'Message a bot',
  usage: 'Usage', quiet: 'Quiet profile', toggleDock: 'Toggle agent dock',
}

export function IslandOptionsExtras({ model }: { model: IslandModel }) {
  const tr = useTr()
  const controls = model.controls
  const [dock] = useAgentDockPreference()
  const state = model.screen.shortcutState
  const bindingKey = JSON.stringify(state?.bindings ?? defaultIslandBindings)
  const [bindings, setBindings] = useState(state?.bindings ?? defaultIslandBindings)
  const [saving, setSaving] = useState(false)
  useEffect(() => { setBindings(JSON.parse(bindingKey)) }, [bindingKey])
  return <>
    <Switch label="Quiet profile" on={controls.quiet} onChange={(quiet) => void model.configure({ quiet })}>Quiet profile</Switch>
    <span className="island-subtitle">{tr('Suppress hover opening and completion notices. Approvals and manual controls remain available.')}</span>
    <Switch label="Quiet while Codync is fullscreen" on={controls.quietFullscreen} onChange={(quietFullscreen) => void model.configure({ quietFullscreen })}>Quiet while Codync is fullscreen</Switch>
    <span className="island-subtitle">{tr('Fullscreen detection applies to Codync windows. Use Quiet profile for other apps.')}</span>
    <label className="island-control-row">{tr('Island text size')}<input aria-label={tr('Island text size')} type="range" min={11} max={18} step={1} value={controls.fontSize} onChange={(event) => void model.configure({ fontSize: Number(event.target.value) })} /><span>{controls.fontSize}</span></label>
    <Switch label="High contrast" on={controls.highContrast} onChange={(highContrast) => void model.configure({ highContrast })}>High contrast</Switch>
    <Switch label="Agent dock" on={dock.shown} onChange={(shown) => model.setDockPreference({ shown })}>Agent dock</Switch>
    <span className="island-subtitle">{tr('A strip of agent seats beside the island. Each seat opens a compact chat with its bot.')}</span>
    <div role="group" aria-label={tr('Dock edge')}>
      <span className="island-subtitle">{tr('Dock edge')}</span>
      <SegmentedChoice<AgentDockEdge> selection={dock.edge} options={[{ id: 'left', label: 'Left' }, { id: 'right', label: 'Right' }]} onChange={(edge) => model.setDockPreference({ edge })} />
    </div>
    <label className="island-control-row">{tr('Dock size')}<input aria-label={tr('Dock size')} type="range" min={AGENT_DOCK_SIZE.min} max={AGENT_DOCK_SIZE.max} step={1} value={dock.sizePercent} onChange={(event) => model.setDockPreference({ sizePercent: Number(event.target.value) })} /><span>{dock.sizePercent}%</span></label>
    <Switch label="Auto-hide dock" on={dock.autoHide} onChange={(autoHide) => model.setDockPreference({ autoHide })}>Auto-hide dock</Switch>
    <span className="island-subtitle">{tr('With auto-hide, the dock tucks into its edge until the pointer reaches it or a chat opens.')}</span>
    <details className="island-workspace-details"><summary>{tr('Island keyboard shortcuts')}</summary>
      <span className="island-subtitle">{tr('Optional global shortcuts. Example: Ctrl+Shift+F8. Leave blank to disable. Approval and app shortcuts are reserved.')}</span>
      {islandShortcutActions.map((action) => <label key={action} className="island-shortcut-field"><span>{tr(labels[action])}</span><input className="island-search-input" aria-label={tr(labels[action])} value={bindings[action]} maxLength={80} placeholder={tr('Disabled')} onChange={(event) => setBindings({ ...bindings, [action]: event.target.value })} />{state?.errors[action] ? <span className="island-control-error" role="status">{state.errors[action]}</span> : null}</label>)}
      <Button disabled={saving} onClick={async () => { setSaving(true); try { await model.setShortcuts(bindings) } finally { setSaving(false) } }}>{tr('Save shortcuts')}</Button>
    </details>
  </>
}
