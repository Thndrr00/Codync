import { useMemo } from 'react'
import {
  cursor_point,
  type AgentCursorDisplay,
  type AgentCursorEvent,
} from '../../lib/cursor-point'

/**
 * The renderer-side marker for one agent move/click (OpenBot computer-use.md). The main process
 * draws the real always-on-top dot; this is the same pure placement used where a renderer owns a
 * surface, so the two never disagree. It is pointer-events:none and fades out; reduced motion turns
 * the fade off. No focus is taken and no point is sent anywhere.
 */
export function AgentCursor({ event, display }: { event: AgentCursorEvent | null; display: AgentCursorDisplay | null }) {
  const point = useMemo(() => cursor_point(event, display), [event, display])
  const reducedMotion = useMemo(
    () => typeof window.matchMedia === 'function' && window.matchMedia('(prefers-reduced-motion: reduce)').matches,
    [],
  )
  if (!point) return null
  return (
    <div
      aria-hidden
      style={{
        position: 'fixed',
        left: point.x,
        top: point.y,
        width: 12,
        height: 12,
        marginLeft: -6,
        marginTop: -6,
        borderRadius: '50%',
        border: '2px solid #fff',
        background: point.click ? '#0a84ff' : '#0a84ffaa',
        boxShadow: '0 1px 4px #0009',
        pointerEvents: 'none',
        transition: reducedMotion ? undefined : 'opacity 200ms ease',
      }}
    />
  )
}
