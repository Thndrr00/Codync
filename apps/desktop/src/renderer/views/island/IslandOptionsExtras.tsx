import { useEffect, useState } from 'react'
import { defaultIslandBindings, islandShortcutActions } from '@shared/island-shortcuts'
import { Button, Switch } from '../../components/Controls'
import { useTr } from '../../lib/use-language'
import type { IslandModel } from './island-model'

const labels = { toggle: 'Show or hide island', search: 'Workspace and search', activity: 'Activity', message: 'Message a bot', usage: 'Usage', quiet: 'Quiet profile' }

export function IslandOptionsExtras({ model }: { model: IslandModel }) {
  const tr = useTr()
  const controls = model.controls
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
    <details className="island-workspace-details"><summary>{tr('Island keyboard shortcuts')}</summary>
      <span className="island-subtitle">{tr('Optional global shortcuts. Example: Ctrl+Shift+F8. Leave blank to disable. Approval and app shortcuts are reserved.')}</span>
      {islandShortcutActions.map((action) => <label key={action} className="island-shortcut-field"><span>{tr(labels[action])}</span><input className="island-search-input" aria-label={tr(labels[action])} value={bindings[action]} maxLength={80} placeholder={tr('Disabled')} onChange={(event) => setBindings({ ...bindings, [action]: event.target.value })} />{state?.errors[action] ? <span className="island-control-error" role="status">{state.errors[action]}</span> : null}</label>)}
      <Button disabled={saving} onClick={async () => { setSaving(true); try { await model.setShortcuts(bindings) } finally { setSaving(false) } }}>{tr('Save shortcuts')}</Button>
    </details>
  </>
}
