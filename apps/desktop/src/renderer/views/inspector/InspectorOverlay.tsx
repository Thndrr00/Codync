import { useEffect, useMemo, useState } from 'react'
import { isWorking } from '@shared/models'
import { redactValue } from '@shared/inspector'
import { useApp } from '../../store/context'
import { useTr } from '../../lib/use-language'
import './inspector.css'

/** The key that opens the Inspector: ⌘/Ctrl + Shift + . */
const TOGGLE = '.'

function initialOpen() {
  const flag = new URLSearchParams(location.search).get('inspector')
  if (flag === '1') return true
  if (flag === '0') return false
  return localStorage.getItem('codync.inspector') === '1'
}

/**
 * Card K15: a development-only overlay with the newest host events (redacted), the turns running now
 * and what each view says. Release builds never mount it (main/ChatWindow gate on `import.meta.env.DEV`).
 */
export function InspectorOverlay() {
  const tr = useTr()
  const app = useApp()
  const [open, setOpen] = useState(initialOpen)
  const [tick, setTick] = useState(0)

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || !event.shiftKey || event.key !== TOGGLE) return
      event.preventDefault()
      setOpen((was) => {
        localStorage.setItem('codync.inspector', was ? '0' : '1')
        return !was
      })
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [])

  useEffect(() => {
    if (!open) return
    const timer = setInterval(() => setTick((n) => n + 1), 1000)
    return () => clearInterval(timer)
  }, [open])

  const stores = useMemo(() => [...app.stores.values()], [app, tick])
  const events = useMemo(
    () => stores.flatMap((store) => store.inspector?.all ?? []).sort((a, b) => a.at - b.at).slice(-60),
    [stores],
  )
  const turns = useMemo(
    () => stores.flatMap((store) => [...store.bots.values()].filter(isWorking).map((bot) => ({ store: store.computer.name, bot }))),
    [stores],
  )
  const views = useMemo<[string, unknown][]>(
    () => (app.selectedStore ? [...app.selectedStore.viewStates.entries()] : []),
    [app, stores, tick],
  )

  if (!open || !import.meta.env.DEV) return null
  return (
    <aside className="inspector" aria-label={tr('Inspector')}>
      <header className="inspector-head">
        <strong>{tr('Inspector')}</strong>
        <span className="inspector-dev">{tr('Development only')}</span>
        <button type="button" onClick={() => { localStorage.setItem('codync.inspector', '0'); setOpen(false) }} aria-label={tr('Close')}>
          ×
        </button>
      </header>
      <section>
        <h4>{tr('Active turns')}</h4>
        {turns.length === 0 ? <p className="inspector-empty">{tr('No bot is working')}</p> : null}
        {turns.map(({ store, bot }) => (
          <p key={`${store}/${bot.id}`} className="inspector-row">
            <span>{bot.name}</span>
            <span className="inspector-dim">{store} · {bot.status}</span>
          </p>
        ))}
      </section>
      <section>
        <h4>{tr('View state')}</h4>
        {views.length === 0 ? <p className="inspector-empty">{tr('No view reported yet')}</p> : null}
        {views.map(([view, state]) => (
          <details key={view} className="inspector-view">
            <summary>{view}</summary>
            <pre>{JSON.stringify(redactValue(state), null, 2)}</pre>
          </details>
        ))}
      </section>
      <section className="inspector-events">
        <h4>{tr('Recent events')}</h4>
        {events.length === 0 ? <p className="inspector-empty">{tr('No events yet')}</p> : null}
        {events.map((event, index) => (
          <details key={`${event.at}-${index}`}>
            <summary>
              <span className="inspector-dim">{new Date(event.at).toLocaleTimeString()}</span> {event.type}
            </summary>
            <pre>{JSON.stringify(event.data, null, 2)}</pre>
          </details>
        ))}
      </section>
    </aside>
  )
}
