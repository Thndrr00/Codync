import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react'
import { IconButtonBase } from '../../components/Overlay'
import { hscrollEdges, hscrollStep } from './hscroll'
import './markdown.css'

/**
 * A wide table or an unwrapped code line scrolls sideways with a fade and an arrow at the edge
 * that still has content (OpenBot 0.72.0). The scroller is the shared element; its content decides
 * whether the arrows appear, including when a later version of the same block gets wider.
 */
export function HScroll({ children }: { children: ReactNode }) {
  const ref = useRef<HTMLDivElement | null>(null)
  const [edges, setEdges] = useState({ left: false, right: false })
  const measure = useCallback(() => {
    const el = ref.current
    if (!el) return
    const next = hscrollEdges(el.scrollLeft, el.clientWidth, el.scrollWidth)
    // Same edges keep the same object: re-measuring after a render must not start another one.
    setEdges((current) => (current.left === next.left && current.right === next.right ? current : next))
  }, [])
  useEffect(() => {
    const el = ref.current
    if (!el) return
    const observer = new ResizeObserver(measure)
    observer.observe(el)
    for (const child of Array.from(el.children)) observer.observe(child)
    measure()
    return () => observer.disconnect()
  }, [measure, children])
  const move = (delta: number) => ref.current?.scrollBy({ left: delta, behavior: 'smooth' })
  return (
    <div className="md-scroll-wrap" data-left={edges.left} data-right={edges.right}>
      <div className="md-scroll" ref={ref} onScroll={measure}>{children}</div>
      {edges.left ? (
        <IconButtonBase className="md-scroll-arrow left" size={22} title="Scroll left" icon="chevron.left"
          onClick={() => move(-hscrollStep(ref.current?.clientWidth ?? 300))} />
      ) : null}
      {edges.right ? (
        <IconButtonBase className="md-scroll-arrow right" size={22} title="Scroll right" icon="chevron.right"
          onClick={() => move(hscrollStep(ref.current?.clientWidth ?? 300))} />
      ) : null}
    </div>
  )
}
