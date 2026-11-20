import { useEffect, useState } from 'react'
import type { IslandRun } from '@shared/island-activity'
import type { IslandBot } from '@shared/island'
import { CharacterAvatar } from '../../components/Avatar'
import { Icon } from '../../components/Icon'
import { usePresence } from '../../components/Overlay'
import { useModel } from '../../lib/observable'
import { useTr } from '../../lib/use-language'
import { useAgentDockPreference } from './agent-dock-preference'
import type { IslandModel } from './island-model'
import { useArrivals } from './island-motion'
import { badgeOf, moodOf, StateBadge, StatusDot } from './island-parts'
import { RunRow } from './IslandActivity'
import { Message } from './IslandViews'
import './agent-dock.css'

/** How long the dock rests before it slides into the edge, when auto-hide is on. */
const TUCK_DELAY_MS = 900

const stillRunning = (run: IslandRun) => run.status === 'running' || run.status === 'queued'

/**
 * The agent dock: a strip of seats at the island window's chosen edge. It only reads the island's
 * live summary and writes the same intents the island already sends; seats open the island's own
 * message view and finished work comes from the island's activity store, so there is no second chat.
 */
export function AgentDock({ model }: { model: IslandModel }) {
  useModel(model)
  const tr = useTr()
  const [pref] = useAgentDockPreference()
  const [hovered, setHovered] = useState(false)
  const [near, setNear] = useState(false)
  const [tucked, setTucked] = useState(false)
  const presence = usePresence(pref.shown, 260)

  // Pointer on the dock, the pointer at the edge strip or an open chat all keep it out.
  useEffect(() => {
    if (!pref.shown || !pref.autoHide || hovered || near || model.dockChatRef) {
      setTucked(false)
      return
    }
    const timer = setTimeout(() => setTucked(true), TUCK_DELAY_MS)
    return () => clearTimeout(timer)
  }, [pref.shown, pref.autoHide, hovered, near, model.dockChatRef])

  const seats = model.dockSeats.flatMap((ref) => {
    const bot = model.summary.bots.find((item) => item.ref === ref)
    return bot ? [bot] : []
  })
  const arrivals = useArrivals(seats.map((bot) => bot.ref))
  const finished = (model.summary.activity?.runs ?? []).filter((run) => !stillRunning(run))
  const resultsFor = (ref: string) => finished.filter((run) => run.ref === ref).slice(-2).reverse()

  return <>
    {pref.shown && pref.autoHide && presence.mounted ? (
      <div
        className={`agent-dock-strip ${pref.edge}`}
        aria-hidden="true"
        onPointerEnter={() => setNear(true)}
        onPointerLeave={() => setNear(false)}
        onPointerDown={() => setTucked(false)}
      />
    ) : null}
    {presence.mounted ? (
      <div
        className={`agent-dock ${pref.edge} ${presence.shown ? 'shown' : ''} ${tucked ? 'tucked' : ''}`}
        style={{ height: `${pref.sizePercent}%`, '--island-text-scale': model.controls.fontSize / 13 } as import('react').CSSProperties}
        role="group"
        aria-label={tr('Agent dock')}
        onPointerEnter={() => setHovered(true)}
        onPointerLeave={() => setHovered(false)}
        onClick={(event) => event.stopPropagation()}
        onContextMenu={(event) => event.stopPropagation()}
        onKeyDown={(event) => event.stopPropagation()}
      >
        <div className="agent-dock-top">
          <span className="island-subtitle">{tr('Agent dock')}</span>
          <button
            className="island-tab"
            title={tr('Hide agent dock')}
            aria-label={tr('Hide agent dock')}
            onClick={() => model.setDockPreference({ shown: false })}
          ><Icon name="xmark" size={12} /></button>
        </div>
        <div className="agent-dock-seats">
          {seats.map((bot) => (
            <DockSeat
              key={bot.ref}
              model={model}
              bot={bot}
              arriving={arrivals.has(bot.ref)}
              active={model.dockChatRef === bot.ref}
              results={resultsFor(bot.ref)}
            />
          ))}
        </div>
        {model.dockChatRef ? <div className="agent-dock-chat"><Message model={model} /></div> : null}
      </div>
    ) : null}
  </>
}

function DockSeat({ model, bot, arriving, active, results }: {
  model: IslandModel
  bot: IslandBot
  arriving: boolean
  active: boolean
  results: IslandRun[]
}) {
  const tr = useTr()
  return (
    <div className="agent-dock-seat-wrap">
      <button
        className={`agent-dock-seat ${active ? 'active' : ''} ${arriving ? 'island-swap' : ''}`}
        title={bot.name}
        aria-label={tr('Chat with {name}').replaceAll('{name}', () => bot.name)}
        aria-pressed={active}
        onClick={() => model.openDockChat(bot.ref)}
      >
        <CharacterAvatar customIcon={bot.customIcon} shape={bot.shape} color={bot.color} size={26} mood={moodOf(bot)} dark />
        <StateBadge badge={badgeOf(bot)} small />
        <StatusDot bot={bot} />
        {bot.unread ? <span className="agent-dock-unread">{bot.unread}</span> : null}
      </button>
      {results.length ? (
        <div className="agent-dock-results">
          {results.map((run) => (
            <div className="island-card agent-dock-result" key={run.id}>
              <RunRow model={model} run={run} />
            </div>
          ))}
        </div>
      ) : null}
    </div>
  )
}
