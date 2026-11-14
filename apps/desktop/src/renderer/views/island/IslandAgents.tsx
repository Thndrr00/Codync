import { useRef } from 'react'
import { agentDockText, type AgentDockState, type AgentDockRow } from '@shared/island-agents'
import { CharacterAvatar, type Mood } from '../../components/Avatar'
import { useTr } from '../../lib/use-language'
import type { IslandModel } from './island-model'
import { useArrivals, useFlip } from './island-motion'
import './island-agents.css'

// The agent dock's view: the bots the island already receives, in the app's roster order, as a
// compact roster. Each row opens that conversation in Codync; nothing here talks to the host.

const moodOf = (state: AgentDockState): Mood => (state === 'needsInput' ? 'needsInput' : state === 'working' ? 'working' : 'idle')

/** What a row says its bot is doing, in the island's words. */
const stateLabel = (tr: (text: string) => string, state: AgentDockState) =>
  tr({ working: 'Working', needsInput: 'Needs you', error: 'Failed', idle: 'Idle' }[state])

export function AgentsView({ model }: { model: IslandModel }) {
  const tr = useTr()
  const rows = model.agents
  const arrivals = useArrivals(rows.map((row) => row.ref))
  const list = useRef<HTMLDivElement>(null)
  useFlip(list)
  return (
    <div className="island-card island-control-panel island-agent-dock">
      <div className="island-agent-dock-head">
        <strong>{tr('Agents')}</strong>
        <span className="island-subtitle">{agentDockText(model.agentLine, tr)}</span>
      </div>
      {rows.length ? (
        <div ref={list} className="island-agent-list">
          {rows.map((row) => (
            <AgentEntry key={row.ref} row={row} arrived={arrivals.has(row.ref)} tr={tr} onOpen={() => model.openBot(row.ref)} />
          ))}
        </div>
      ) : (
        <>
          <span className="island-subtitle">{tr('No agents on this computer yet. Create a bot to see it here.')}</span>
          <button className="island-option" onClick={() => model.newBot()}>{tr('New bot')}</button>
        </>
      )}
    </div>
  )
}

function AgentEntry({ row, arrived, tr, onOpen }: { row: AgentDockRow; arrived: boolean; tr: (text: string) => string; onOpen: () => void }) {
  const state = stateLabel(tr, row.state)
  return (
    <button
      data-flip={row.ref}
      className={`island-agent-entry ${arrived ? 'island-swap' : ''}`}
      onClick={onOpen}
      title={row.activity ? `${row.name} · ${row.activity}` : row.name}
      aria-label={`${row.name}, ${state}${row.unread ? `, ${row.unread} ${tr('Unread')}` : ''}`}
    >
      <CharacterAvatar customIcon={row.customIcon} shape={row.shape} color={row.color} size={20} mood={moodOf(row.state)} dark />
      <span className="island-agent-entry-name truncate">{row.name}</span>
      <span className="island-agent-entry-activity">{row.activity}</span>
      {row.unread ? <span className="island-agent-unread" aria-hidden>{row.unread > 99 ? '99+' : row.unread}</span> : null}
      <span className="island-agent-state" data-state={row.state}>{state}</span>
    </button>
  )
}
