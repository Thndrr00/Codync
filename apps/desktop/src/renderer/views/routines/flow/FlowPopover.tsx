import { useLayoutEffect, useRef, useState, type CSSProperties, type ReactNode, type RefObject } from 'react'
import { createPortal } from 'react-dom'
import { ModalHeader, useEscape, usePresence } from '../../../components/Overlay'
import { useTr } from '../../../lib/use-language'

/**
 * A small panel under a toolbar button, with Codync's own Motion (shared `usePresence`) and Escape.
 * The workflow's Answer fields and Check controls live here so the step itself stays a single prompt.
 */
export function FlowPopover({ open, onClose, anchor, title, width = 360, children }: {
  open: boolean
  onClose: () => void
  anchor: RefObject<HTMLElement | null>
  title: string
  width?: number
  children: ReactNode
}) {
  const tr = useTr()
  const { mounted, shown } = usePresence(open, 120)
  const layer = useRef<HTMLDivElement>(null)
  const panel = useRef<HTMLDivElement>(null)
  const [place, setPlace] = useState<CSSProperties>({ top: 0, left: 0 })
  useEscape(open, onClose, layer)
  useLayoutEffect(() => {
    if (!open || !mounted || !anchor.current || !panel.current) return
    const from = anchor.current.getBoundingClientRect()
    const height = panel.current.offsetHeight
    const below = window.innerHeight - from.bottom > height + 12
    setPlace({
      top: below ? from.bottom + 6 : Math.max(8, from.top - height - 6),
      left: Math.min(Math.max(8, from.left), window.innerWidth - width - 8),
      transformOrigin: below ? 'top left' : 'bottom left',
    })
  }, [open, mounted, anchor, width])
  if (!mounted) return null
  return createPortal(
    <div
      ref={layer}
      className={`flow-popover ${shown ? 'shown' : ''}`}
      onMouseDown={(event) => { if (event.target === event.currentTarget) onClose() }}
    >
      <div ref={panel} className="flow-popover-card" style={{ ...place, width }}>
        <ModalHeader title={tr(title)} onClose={onClose} />
        <div className="flow-popover-body">{open ? children : null}</div>
      </div>
    </div>,
    document.body,
  )
}
