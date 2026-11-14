import { useEffect } from 'react'
import type { Person } from '@shared/models'
import { avatarColor } from '../../lib/theme'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'
import './presence.css'

/** How often the view asks who else is in the chat; presence is thrown away thirty seconds
 * after a device last looked, so this keeps this person's own place alive while reading. */
const POLL = 3_000

/**
 * Who else is in this chat right now, over the message box: their initials in their colour, and
 * "Ana is typing…" while a key comes from them. Port of OpenBot's team presence line
 * (`TeamPresenceMember`, `TeamPersonAvatar`), with the devices paired to this computer as the
 * people. Nothing shows while this is the only one here.
 */
export function PresenceRow({ botId }: { botId: string }) {
  const store = useStore()
  const tr = useTr()

  useEffect(() => {
    const look = () => void store.presence(botId, store.isTyping(botId)).catch(() => {})
    look()
    const timer = setInterval(look, POLL)
    // Back from another app or a hidden window: show the present at once.
    const wake = () => { if (!document.hidden) look() }
    document.addEventListener('visibilitychange', wake)
    window.addEventListener('focus', wake)
    return () => {
      clearInterval(timer)
      document.removeEventListener('visibilitychange', wake)
      window.removeEventListener('focus', wake)
    }
  }, [store, botId])

  const people = store.people(botId)
  if (!people.length) return null
  const typing = people.filter((p) => p.typing)
  const shown = people.slice(0, 3)
  return (
    <div className="presence-row" role="status">
      <span className="presence-faces" aria-hidden="true">
        {shown.map((p) => (
          <span key={p.key} className={p.typing ? 'presence-face typing' : 'presence-face'} style={{ background: avatarColor(p.color) }}>
            {initials(p.name)}
          </span>
        ))}
      </span>
      <span className="presence-line">{typing.length ? typingLine(typing, tr) : hereLine(people, tr)}</span>
    </div>
  )
}

/** Up to two letters of a name, for its small face. */
function initials(name: string): string {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((word) => [...word][0]?.toUpperCase() ?? '')
    .join('')
}

/** "Ana is typing…" / "Ana and Bo are typing…" / "Ana, Bo and Cy are typing…". */
function typingLine(people: Person[], tr: ReturnType<typeof useTr>): string {
  const [a, b] = people
  if (!b) return tr('{name} is typing…').replaceAll('{name}', () => a!.name)
  if (people.length === 2) return tr('{a} and {b} are typing…').replaceAll('{a}', () => a!.name).replaceAll('{b}', () => b.name)
  return tr('{names} are typing…').replaceAll('{names}', () => people.map((p) => p.name).join(', '))
}

/** "Ana is here" / "Ana and Bo are here" / "Ana, Bo and 2 more are here". */
function hereLine(people: Person[], tr: ReturnType<typeof useTr>): string {
  const [a, b] = people
  if (!b) return tr('{name} is here').replaceAll('{name}', () => a!.name)
  if (people.length === 2) return tr('{a} and {b} are here').replaceAll('{a}', () => a!.name).replaceAll('{b}', () => b.name)
  return tr('{names} and {count} more are here')
    .replaceAll('{names}', () => people.slice(0, 2).map((p) => p.name).join(', '))
    .replaceAll('{count}', () => String(people.length - 2))
}
