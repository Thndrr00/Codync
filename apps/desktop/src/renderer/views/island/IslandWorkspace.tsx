import { useEffect, useRef, useState } from 'react'
import { elapsedIslandRun, searchIslandBots } from '@shared/island-workspace'
import { Button, SearchField } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { useTr } from '../../lib/use-language'
import type { IslandModel } from './island-model'
import { IslandPanel } from './IslandPanel'

const searchable = (value: string) => value.normalize('NFD').replace(/\p{M}/gu, '').toLocaleLowerCase()

/** Search and inspect existing host-owned workspace snapshots without sending message bodies. */
export function WorkspaceView({ model }: { model: IslandModel }) {
  const tr = useTr()
  const [query, setQuery] = useState('')
  const input = useRef<HTMLInputElement | null>(null)
  const focus = model.focus
  const selected = focus?.ref
  const workspace = model.summary.workspace?.ref === selected ? model.summary.workspace : null
  const bots = searchIslandBots(model.summary.bots, query)
  const terms = searchable(query.trim()).split(/\s+/).filter(Boolean)
  const commands = [
    { title: tr('Message a bot'), run: () => model.setTab('message') },
    { title: tr('Usage'), run: () => model.setTab('usage') },
    { title: tr('Activity'), run: () => model.setTab('activity') },
    { title: tr('Approvals'), run: () => model.setTab('approvals') },
    { title: tr('Voice call'), run: () => model.setTab('status') },
    { title: tr('Connection and health'), run: () => model.setTab('status') },
    { title: tr('Channels'), run: () => model.action({ kind: 'openChannels' }) },
    { title: tr('Diagnostics'), run: () => model.action({ kind: 'openDiagnostics' }) },
    { title: tr('Island options'), run: () => model.setTab('options') },
  ].filter((command) => terms.every((term) => searchable(command.title).includes(term)))
  const elapsed = focus?.status === 'working' ? elapsedIslandRun(focus.startedAt, null, Date.now()) : null

  useEffect(() => { input.current?.focus() }, [])
  useEffect(() => { if (selected) model.action({ kind: 'loadWorkspace', ref: selected }) }, [model, selected])

  return <IslandPanel>
    <div className="island-control-heading"><strong>{tr('Workspace')}</strong><button className="island-tab" disabled={!selected || workspace?.loading} title={tr('Refresh workspace')} aria-label={tr('Refresh workspace')} onClick={() => selected && model.action({ kind: 'loadWorkspace', ref: selected })}><Icon name="arrow.clockwise" size={13} /></button></div>
    <SearchField value={query} onChange={setQuery} prompt="Search bots, projects and actions" inputRef={input} />
    <div className="island-control-actions" role="group" aria-label={tr('Actions')}>
      {commands.map((command) => <Button key={command.title} onClick={command.run}>{command.title}</Button>)}
    </div>
    <strong>{tr('Bots')}</strong>
    <div className="island-control-actions" role="group" aria-label={tr('Select bot')}>
      {bots.map((bot) => <Button key={bot.ref} kind={selected === bot.ref ? 'primary' : 'secondary'} aria-pressed={selected === bot.ref} title={`${bot.name} · ${bot.backend} · ${bot.project || bot.folder}`} onClick={() => {
        model.selectFocus(bot.ref)
        if (selected === bot.ref) model.action({ kind: 'loadWorkspace', ref: bot.ref })
      }}>{bot.name} · {bot.folder}</Button>)}
    </div>
    {!bots.length ? <span className="island-subtitle">{tr('No matching bots')}</span> : null}
    {focus ? <>
      <div className="island-control-heading"><strong>{focus.name}</strong><Button onClick={() => model.openBot(focus.ref)}>{tr('Open conversation')}</Button></div>
      {focus.project ? <span className="island-subtitle">{focus.project}</span> : null}
      {focus.status === 'working' ? <span className="island-subtitle">{tr('Elapsed')}: {elapsed === null ? tr('Not reported') : `${elapsed} ${tr('seconds')}`}</span> : null}
      {!workspace || workspace.loading ? <span className="island-subtitle" role="status">{tr('Loading workspace…')}</span> : null}
    </> : <span className="island-subtitle">{tr('No bots yet')}</span>}
    {model.actionError ? <span className="island-control-error" role="status">{tr(model.actionError)}</span> : null}
    {workspace?.error ? <span className="island-control-error" role="status">{workspace.error}</span> : null}
    {workspace?.actionError ? <span className="island-control-error" role="alert">{workspace.actionError}</span> : null}
    {workspace ? <>
      <strong>{tr('Git')}</strong>
      {workspace.git ? <>
        <div className="island-control-row"><span>{tr('Branch')}</span><span>{workspace.git.branch ?? tr('Not reported')}</span></div>
        <div className="island-control-row"><span>{tr('Changed files')}</span><span>{workspace.git.changes ?? tr('Not reported')}</span></div>
        {workspace.git.error ? <span className="island-control-error">{workspace.git.error}</span> : null}
      </> : <span className="island-subtitle">{tr('Git status unavailable')}</span>}
      <strong>{tr('Recent artifacts')}</strong>
      {workspace.artifacts.length ? workspace.artifacts.map((artifact) => <button key={artifact.id} className="island-control-row" disabled={workspace.loading} onClick={() => model.action({ kind: 'openArtifact', ref: workspace.ref, id: artifact.id })}><span>{artifact.title}</span><Icon name="arrow.up.right.square" size={12} /></button>) : <span className="island-subtitle">{tr('No recent artifacts')}</span>}
      <strong>{tr('Registered files')}</strong>
      {workspace.files.length ? workspace.files.map((file) => <button key={`${file.entryId}/${file.id}`} className="island-control-row" disabled={workspace.loading} onClick={() => model.action({ kind: 'openFile', ref: workspace.ref, id: file.id, entryId: file.entryId })}><span>{file.name}</span><Icon name="arrow.up.right.square" size={12} /></button>) : <span className="island-subtitle">{tr('No registered files')}</span>}
      <strong>{tr('Completed edits')}</strong>
      {workspace.changes.length ? workspace.changes.map((change) => <details key={change.entryId} className="island-change-preview">
        <summary>{tr('Changes')} · {new Date(change.at).toLocaleString()} · {change.diffs.length} {tr('files')}</summary>
        {change.diffs.map((diff, index) => <div key={`${diff.path}/${index}`}>
          <div className="island-control-row"><span>{diff.path}</span><span>+{diff.added} −{diff.removed}</span></div>
          <pre className="island-diff-preview">{diff.patch || tr('No patch available')}</pre>
        </div>)}
      </details>) : <span className="island-subtitle">{tr('No completed edits')}</span>}
      <strong>{tr('Queued messages')}</strong>
      {workspace.queue.length ? workspace.queue.map((entry, index) => <div className="island-control-row" key={entry.id}>
        <span>{index + 1}. {tr('Queued message')} · {new Date(entry.at).toLocaleTimeString()}</span>
        <Button kind="secondary" disabled={workspace.busy || workspace.loading} onClick={() => model.action({ kind: 'cancelQueued', ref: workspace.ref, id: entry.id })}>{tr('Cancel')}</Button>
      </div>) : <span className="island-subtitle">{tr('No queued messages')}</span>}
      <strong>{workspace.computerName ?? tr('This computer')} · {tr('Last 24 hours')}</strong>
      {workspace.day ? <>
        <div className="island-control-row"><span>{tr('Messages you sent')}</span><span>{workspace.day.messages}</span></div>
        <div className="island-control-row"><span>{tr('Replies from your bots')}</span><span>{workspace.day.replies}</span></div>
        <div className="island-control-row"><span>{tr('Tool calls')}</span><span>{workspace.day.tools}</span></div>
        <div className="island-control-row"><span>{tr('File edits')}</span><span>{workspace.day.fileEdits}</span></div>
      </> : <span className="island-subtitle">{tr('Summary unavailable')}</span>}
    </> : null}
    <strong>{tr('Saved drafts')}</strong>
    {model.summary.bots.some((bot) => bot.hasDraft) ? model.summary.bots.filter((bot) => bot.hasDraft).map((bot) => <button className="island-control-row" key={bot.ref} onClick={() => model.action({ kind: 'resumeDraft', ref: bot.ref })}><span>{bot.name}</span><span>{tr('Resume draft')}</span></button>) : <span className="island-subtitle">{tr('No saved drafts')}</span>}
    <div className="island-control-heading"><strong>{tr('Notice history')}</strong><Button kind="secondary" disabled={!model.history.items.length} onClick={() => model.clearHistory()}>{tr('Clear history')}</Button></div>
    {model.history.items.length ? model.history.items.map((item) => <div className="island-control-row vertical" key={item.key}>
      {item.ref ? <button className="island-control-row" onClick={() => { if (item.ref) model.openBot(item.ref) }}><span>{item.text}</span><span>×{item.count}</span></button> : <div className="island-control-row"><span>{item.text}</span><span>×{item.count}</span></div>}
      <time className="island-subtitle">{new Date(item.at).toLocaleString()}</time>
    </div>) : <span className="island-subtitle">{tr('No saved notices')}</span>}
  </IslandPanel>
}
