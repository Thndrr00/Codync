import { useCallback, useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type PointerEvent as ReactPointerEvent, type RefObject } from 'react'
import { reduceMotion } from '../lib/theme'
import { useTr } from '../lib/use-language'
import { Icon } from './Icon'
import { AnchoredMenu } from './Menu'
import './tab-strip.css'

export interface StripTab<T extends string> {
  id: T
  /** The tab's own title, shown as typed (not translated) in the tab and the overflow menu. */
  title: string
  icon?: string
}

const EDGE_ZONE_PX = 40
const EDGE_SPEED_PX = 14

interface RowState<T extends string> {
  hidden: T[]
  fadeStart: boolean
  fadeEnd: boolean
  frozen: number
}

/**
 * The scroll, measure and drag-edge logic of one fixed-width tab row. Tabs out of view are listed
 * in an overflow menu; a close freezes the width until the pointer leaves the row, so the next X
 * comes under the pointer; a tab held near an end scrolls the row, so it can be moved out of view.
 */
export function useTabRow<T extends string>(listRef: RefObject<HTMLDivElement | null>, tabs: readonly StripTab<T>[]) {
  const [row, setRow] = useState<RowState<T>>({ hidden: [], fadeStart: false, fadeEnd: false, frozen: 0 })
  const frame = useRef(0)
  // The ids joined are the row's identity: a caller may pass a fresh `tabs` array each render.
  const key = tabs.map((t) => t.id).join('\u0000')
  const current = useRef(tabs)
  current.current = tabs

  const measure = useCallback(() => {
    frame.current = 0
    const list = listRef.current
    if (!list) return
    const start = list.scrollLeft
    const end = start + list.clientWidth
    const hidden = current.current
      .filter((_, i) => {
        const item = list.children[i] as HTMLElement | undefined
        return !!item && (item.offsetLeft < start - 1 || item.offsetLeft + item.offsetWidth > end + 1)
      })
      .map((t) => t.id)
    setRow((prev) => {
      const same = prev.hidden.length === hidden.length && prev.hidden.every((id, i) => id === hidden[i])
      return {
        hidden: same ? prev.hidden : hidden,
        fadeStart: start > 1,
        fadeEnd: end < list.scrollWidth - 1,
        frozen: prev.frozen,
      }
    })
  }, [listRef])

  const queueMeasure = useCallback(() => {
    if (!frame.current) frame.current = requestAnimationFrame(measure)
  }, [measure])

  const reveal = useCallback((id: T) => {
    const list = listRef.current
    const item = list?.children[current.current.findIndex((t) => t.id === id)] as HTMLElement | undefined
    if (!list || !item) return
    const inset = Number.parseFloat(getComputedStyle(list).paddingInlineStart) || 0
    const left = item.offsetLeft - inset
    const right = item.offsetLeft + item.offsetWidth + inset
    const behavior: ScrollBehavior = reduceMotion() ? 'auto' : 'smooth'
    if (left < list.scrollLeft) list.scrollTo({ left, behavior })
    else if (right > list.scrollLeft + list.clientWidth) list.scrollTo({ left: right - list.clientWidth, behavior })
  }, [listRef])

  const scrollAtEdge = useCallback((x: number) => {
    const list = listRef.current
    if (!list) return
    const bounds = list.getBoundingClientRect()
    const speed = x < bounds.left + EDGE_ZONE_PX
      ? -(1 - Math.max(0, x - bounds.left) / EDGE_ZONE_PX)
      : x > bounds.right - EDGE_ZONE_PX
        ? 1 - Math.max(0, bounds.right - x) / EDGE_ZONE_PX
        : 0
    if (speed) list.scrollLeft += speed * EDGE_SPEED_PX
  }, [listRef])

  const freeze = useCallback((id: T) => {
    const list = listRef.current
    const item = list?.children[current.current.findIndex((t) => t.id === id)] as HTMLElement | undefined
    const width = item?.offsetWidth
    if (width) setRow((prev) => ({ ...prev, frozen: width }))
  }, [listRef])

  const unfreeze = useCallback(() => setRow((prev) => (prev.frozen ? { ...prev, frozen: 0 } : prev)), [])

  useEffect(() => {
    const list = listRef.current
    if (!list) return
    const onWheel = (e: WheelEvent) => {
      if (e.ctrlKey || Math.abs(e.deltaY) <= Math.abs(e.deltaX) || list.scrollWidth <= list.clientWidth) return
      e.preventDefault()
      list.scrollLeft += e.deltaY
    }
    const sizes = new ResizeObserver(queueMeasure)
    sizes.observe(list)
    list.addEventListener('wheel', onWheel, { passive: false })
    list.addEventListener('scroll', queueMeasure, { passive: true })
    list.addEventListener('transitionend', queueMeasure)
    queueMeasure()
    return () => {
      sizes.disconnect()
      cancelAnimationFrame(frame.current)
      frame.current = 0
      list.removeEventListener('wheel', onWheel)
      list.removeEventListener('scroll', queueMeasure)
      list.removeEventListener('transitionend', queueMeasure)
    }
  }, [listRef, queueMeasure])

  useLayoutEffect(() => { queueMeasure() }, [key, queueMeasure])
  useEffect(() => { if (row.frozen) queueMeasure() }, [row.frozen, queueMeasure])

  return { ...row, reveal, measure: queueMeasure, scrollAtEdge, freeze, unfreeze }
}

/**
 * A row of equal-width tabs with a per-tab X. The X shows on the selected tab and on the tab under
 * the pointer; middle click closes a tab; with too many tabs the row scrolls and a menu at the end
 * lists the hidden ones. A held tab dragged near an end scrolls the row, so it can move out of view.
 */
export function TabStrip<T extends string>({ tabs, active, label, closeLabel = 'Close tab', hiddenLabel = 'Hidden tabs', onSelect, onClose, onReorder }: {
  tabs: readonly StripTab<T>[]
  active: T
  label: string
  closeLabel?: string
  hiddenLabel?: string
  onSelect: (id: T) => void
  onClose: (id: T) => void
  /** Called with the dragged and the target tab when a held tab is dropped on another one. */
  onReorder?: (from: T, to: T) => void
}) {
  const tr = useTr()
  const listRef = useRef<HTMLDivElement>(null)
  const menuRef = useRef<HTMLButtonElement>(null)
  const [menuOpen, setMenuOpen] = useState(false)
  const row = useTabRow(listRef, tabs)
  const drag = useRef<{ id: T; x: number; frame: number; moved: boolean } | null>(null)
  const suppressClick = useRef(false)

  const stopDrag = useCallback(() => {
    const d = drag.current
    if (!d) return
    cancelAnimationFrame(d.frame)
    drag.current = null
  }, [])
  useEffect(() => () => stopDrag(), [stopDrag])

  const closeWithPointer = useCallback((id: T) => {
    row.freeze(id)
    onClose(id)
  }, [row.freeze, onClose])

  const onPointerDown = (e: ReactPointerEvent<HTMLDivElement>, id: T) => {
    if (e.button !== 0) return
    const d = { id, x: e.clientX, frame: 0, moved: false }
    drag.current = d
    e.currentTarget.setPointerCapture(e.pointerId)
    const step = () => {
      const held = drag.current
      if (!held) return
      row.scrollAtEdge(held.x)
      held.frame = requestAnimationFrame(step)
    }
    d.frame = requestAnimationFrame(step)
  }
  const onPointerMove = (e: ReactPointerEvent<HTMLDivElement>) => {
    const d = drag.current
    if (!d) return
    if (Math.abs(e.clientX - d.x) > 4) d.moved = true
    d.x = e.clientX
  }
  const onPointerUp = (e: ReactPointerEvent<HTMLDivElement>) => {
    const d = drag.current
    if (!d) return
    if (d.moved && onReorder) {
      const target = document.elementFromPoint(e.clientX, e.clientY)?.closest<HTMLElement>('.tab-strip-tab')
      const to = target?.dataset.id as T | undefined
      if (to && to !== d.id) onReorder(d.id, to)
    }
    suppressClick.current = d.moved
    stopDrag()
    // A drag may end without a click reaching the tab's button; never leave the flag set for the
    // next real click.
    if (d.moved) requestAnimationFrame(() => { suppressClick.current = false })
  }

  return (
    <div className="tab-strip">
      <div className="tab-strip-viewport" data-fade-start={row.fadeStart ? '' : undefined} data-fade-end={row.fadeEnd ? '' : undefined}>
        <div
          ref={listRef}
          className="tab-strip-tabs"
          role="tablist"
          aria-label={tr(label)}
          data-frozen={row.frozen ? '' : undefined}
          style={row.frozen ? ({ '--tab-strip-frozen': `${row.frozen}px` } as CSSProperties) : undefined}
          onPointerLeave={() => { row.unfreeze(); stopDrag() }}
        >
          {tabs.map((tab) => (
            <div
              key={tab.id}
              className={`tab-strip-tab${tab.id === active ? ' selected' : ''}`}
              data-id={tab.id}
              onPointerDown={(e) => onPointerDown(e, tab.id)}
              onPointerMove={onPointerMove}
              onPointerUp={onPointerUp}
              onPointerCancel={stopDrag}
            >
              <button
                type="button"
                role="tab"
                aria-selected={tab.id === active}
                className="tab-strip-select"
                title={tab.title}
                onClick={() => {
                  if (suppressClick.current) { suppressClick.current = false; return }
                  onSelect(tab.id)
                }}
                onAuxClick={(e) => { if (e.button === 1) closeWithPointer(tab.id) }}
              >
                {tab.icon ? <Icon name={tab.icon} size={12} /> : null}
                <span className="tab-strip-title">{tab.title}</span>
              </button>
              <button
                type="button"
                tabIndex={-1}
                className="tab-strip-close"
                title={tr(closeLabel)}
                aria-label={tr(closeLabel)}
                onMouseDown={(e) => e.stopPropagation()}
                onClick={(e) => { e.stopPropagation(); closeWithPointer(tab.id) }}
              >
                <Icon name="xmark" size={9} weight="bold" />
              </button>
            </div>
          ))}
        </div>
      </div>
      {row.hidden.length ? (
        <button
          ref={menuRef}
          type="button"
          className="tab-strip-more"
          title={tr(hiddenLabel)}
          aria-label={tr(hiddenLabel)}
          aria-haspopup="menu"
          aria-expanded={menuOpen}
          onClick={() => setMenuOpen(true)}
        >
          <Icon name="chevron.down" size={11} weight="semibold" />
        </button>
      ) : null}
      <AnchoredMenu
        open={menuOpen}
        onClose={() => setMenuOpen(false)}
        anchor={menuRef}
        items={() => row.hidden.map((id) => {
          const tab = tabs.find((t) => t.id === id)!
          return { title: tab.title, verbatim: true, action: () => { onSelect(id); row.reveal(id) } }
        })}
      />
    </div>
  )
}
