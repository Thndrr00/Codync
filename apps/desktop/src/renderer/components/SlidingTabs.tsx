import { useLayoutEffect, useRef, useState, type ReactNode } from 'react'

export interface SlidingTabItem<T extends string> {
  id: T
  label: ReactNode
  badge?: string | number
}

/** Idea 17: Dynamic sliding pill tab indicator using ResizeObserver + CSS custom properties. */
export function SlidingTabs<T extends string>({
  items,
  value,
  onChange,
  className = '',
  ariaLabel,
}: {
  items: readonly SlidingTabItem<T>[]
  value: T
  onChange: (next: T) => void
  className?: string
  ariaLabel?: string
}) {
  const containerRef = useRef<HTMLDivElement>(null)
  const tabRefs = useRef<Map<T, HTMLButtonElement>>(new Map())
  const [pill, setPill] = useState<{ x: number; w: number; ready: boolean }>({
    x: 0,
    w: 0,
    ready: false,
  })

  useLayoutEffect(() => {
    const update = () => {
      const container = containerRef.current
      const activeEl = tabRefs.current.get(value)
      if (!container || !activeEl) return
      const cRect = container.getBoundingClientRect()
      const aRect = activeEl.getBoundingClientRect()
      setPill({
        x: aRect.left - cRect.left,
        w: aRect.width,
        ready: true,
      })
    }
    update()
    const ro = new ResizeObserver(update)
    if (containerRef.current) ro.observe(containerRef.current)
    for (const el of tabRefs.current.values()) ro.observe(el)
    window.addEventListener('resize', update)
    return () => {
      ro.disconnect()
      window.removeEventListener('resize', update)
    }
  }, [value, items])

  return (
    <div
      ref={containerRef}
      role="tablist"
      aria-label={ariaLabel}
      className={`t-tabs ${className}`.trim()}
      style={
        {
          '--pill-x': `${pill.x}px`,
          '--pill-w': `${pill.w}px`,
        } as React.CSSProperties
      }
    >
      <span aria-hidden className={`t-tabs-pill ${pill.ready ? 'is-ready' : ''}`} />
      {items.map((item) => {
        const active = item.id === value
        return (
          <button
            key={item.id}
            ref={(el) => {
              if (el) tabRefs.current.set(item.id, el)
              else tabRefs.current.delete(item.id)
            }}
            type="button"
            role="tab"
            aria-selected={active}
            onClick={() => onChange(item.id)}
            className={`t-tab ${active ? 'is-active' : ''}`}
          >
            <span>{item.label}</span>
            {item.badge !== undefined && <span className="t-tab-badge">{item.badge}</span>}
          </button>
        )
      })}
    </div>
  )
}
