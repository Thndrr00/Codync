import { useCallback, useEffect, useRef, useState, useSyncExternalStore } from 'react'
import type { Bot } from '@shared/models'
import { Text } from '../components/Text'
import { rosterElapsed } from '../lib/roster-query'

const listeners = new Set<() => void>()
let timer: ReturnType<typeof setInterval> | null = null
let now = Date.now()
const snapshot = () => now
function subscribe(listener: () => void) {
  listeners.add(listener)
  now = Date.now()
  if (!timer) timer = setInterval(() => { now = Date.now(); for (const notify of listeners) notify() }, 1000)
  return () => { listeners.delete(listener); if (!listeners.size && timer) { clearInterval(timer); timer = null } }
}

export function RosterElapsed({ bot, online }: { bot: Bot; online: boolean }) {
  const element = useRef<HTMLSpanElement>(null)
  const [visible, setVisible] = useState(false)
  useEffect(() => {
    const observer = new IntersectionObserver(([entry]) => setVisible(entry.isIntersecting))
    if (element.current) observer.observe(element.current)
    return () => observer.disconnect()
  }, [])
  const listen = useCallback((listener: () => void) => online && visible ? subscribe(listener) : () => {}, [online, visible])
  const time = useSyncExternalStore(listen, snapshot)
  return <span ref={element}><Text size="caption" tone="secondary">{online ? rosterElapsed(bot.startedAt, time) : 'Offline · last known activity'}</Text></span>
}
