import { useCallback, useRef, type PointerEvent } from 'react'

/** Idea 23: 3D perspective card hover tilt + radial glare tracking via CSS custom properties. */
export function useCardTilt(maxDeg = 5) {
  const ref = useRef<HTMLDivElement>(null)

  const onPointerMove = useCallback(
    (e: PointerEvent<HTMLDivElement>) => {
      const el = ref.current
      if (!el) return
      const rect = el.getBoundingClientRect()
      if (!rect.width || !rect.height) return
      const px = (e.clientX - rect.left) / rect.width
      const py = (e.clientY - rect.top) / rect.height
      const ry = ((px - 0.5) * 2 * maxDeg).toFixed(2)
      const rx = ((0.5 - py) * 2 * maxDeg).toFixed(2)
      el.style.setProperty('--rx', `${rx}deg`)
      el.style.setProperty('--ry', `${ry}deg`)
      el.style.setProperty('--gx', `${(px * 100).toFixed(1)}%`)
      el.style.setProperty('--gy', `${(py * 100).toFixed(1)}%`)
    },
    [maxDeg],
  )

  const onPointerLeave = useCallback(() => {
    const el = ref.current
    if (!el) return
    el.style.setProperty('--rx', '0deg')
    el.style.setProperty('--ry', '0deg')
  }, [])

  return { ref, onPointerMove, onPointerLeave }
}
