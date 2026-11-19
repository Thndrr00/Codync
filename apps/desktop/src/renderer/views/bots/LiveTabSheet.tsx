import { useCallback, useEffect, useRef, useState, type KeyboardEvent, type PointerEvent, type WheelEvent } from 'react'
import { Button, Spinner } from '../../components/Controls'
import { ModalHeader, Sheet, SheetPage } from '../../components/Overlay'
import { HStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'
import './live-tab.css'

/** The desk polls the host's bounded frame slot at the same rate the host captures (2 fps). */
const POLL_MS = 500

interface LiveFrame {
  image: string | null
  title: string
  url: string
  controlling: boolean
}

function reasonText(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason)
}

function mouseButton(button: number): string {
  return button === 2 ? 'right' : button === 1 ? 'middle' : 'left'
}

/** The key names `Key::parse` on the host understands; anything else is left to the app. */
function keyLabel(event: KeyboardEvent<HTMLElement>): string | null {
  const modifiers = [
    event.altKey ? 'Alt' : null,
    event.ctrlKey ? 'Control' : null,
    event.metaKey ? 'Meta' : null,
    event.shiftKey ? 'Shift' : null,
  ].filter((part): part is string => part !== null)
  const key = event.key
  let name: string | null = null
  if (key === ' ') name = 'Space'
  else if (key.length === 1) name = key
  else if (/^(Enter|Tab|Escape|Backspace|Delete|Home|End|PageUp|PageDown|Insert|Arrow(Up|Down|Left|Right)|F([1-9]|1[0-2]))$/.test(key)) name = key
  return name ? [...modifiers, name].join('+') : null
}

/**
 * A sheet that shows one of this bot's open browser tabs and, once a person takes control, sends
 * their pointer and key input to that same tab through the host's existing CDP input path. The host
 * captures at most two JPEG frames a second; the sheet only draws what it polls. Closing the sheet
 * stops the stream and gives control back. The stream is never logged or sent to analytics.
 */
export function LiveTabSheet({ botId, open, close }: { botId: string; open: boolean; close: () => void }) {
  const tr = useTr()
  const store = useStore()
  const [frame, setFrame] = useState<LiveFrame | null>(null)
  const [controlling, setControlling] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const polling = useRef(false)
  const lastMove = useRef(0)
  const image = useRef<HTMLImageElement>(null)

  const call = useCallback(async <T,>(method: string, body: object = {}) => {
    const client = await store.ready()
    return client.call<T>(method, { botId, ...body }, 8_000)
  }, [store, botId])

  useEffect(() => {
    if (!open) return
    let cancelled = false
    setError(null)
    void (async () => {
      try {
        const first = await call<LiveFrame>('browserLiveStart')
        if (cancelled) return
        setFrame(first)
        setControlling(first.controlling)
      } catch (reason) {
        if (!cancelled) setError(reasonText(reason))
      }
    })()
    const timer = setInterval(() => {
      if (polling.current) return
      polling.current = true
      void call<LiveFrame>('browserLiveFrame')
        .then((next) => { if (!cancelled) { setFrame(next); setControlling(next.controlling) } })
        .catch((reason) => { if (!cancelled) setError(reasonText(reason)) })
        .finally(() => { polling.current = false })
    }, POLL_MS)
    return () => {
      cancelled = true
      clearInterval(timer)
      void call('browserLiveStop').catch(() => {})
    }
  }, [open, call])

  const send = useCallback((event: object) => {
    void call('browserLiveInput', { event }).catch((reason) => setError(reasonText(reason)))
  }, [call])

  const point = (event: { clientX: number; clientY: number }) => {
    const node = image.current
    if (!node) return { x: 0, y: 0 }
    const rect = node.getBoundingClientRect()
    // CDP takes CSS pixels from the page's origin; map the drawn image back to the captured one.
    const scaleX = rect.width ? node.naturalWidth / rect.width : 1
    const scaleY = rect.height ? node.naturalHeight / rect.height : 1
    return { x: (event.clientX - rect.left) * scaleX, y: (event.clientY - rect.top) * scaleY }
  }

  const toggle = async () => {
    if (busy) return
    setBusy(true)
    setError(null)
    try {
      const next = await call<{ controlling: boolean }>(controlling ? 'browserLiveRelease' : 'browserLiveTakeover')
      setControlling(next.controlling)
    } catch (reason) {
      setError(reasonText(reason))
    } finally {
      setBusy(false)
    }
  }

  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (!controlling) return
    event.currentTarget.setPointerCapture(event.pointerId)
    const { x, y } = point(event)
    send({ kind: 'mouseDown', x, y, button: mouseButton(event.button), buttons: event.buttons, clickCount: Math.min(2, Math.max(1, event.detail)) })
  }
  const onPointerUp = (event: PointerEvent<HTMLDivElement>) => {
    if (!controlling) return
    const { x, y } = point(event)
    send({ kind: 'mouseUp', x, y, button: mouseButton(event.button), clickCount: Math.min(2, Math.max(1, event.detail)) })
  }
  const onPointerMove = (event: PointerEvent<HTMLDivElement>) => {
    if (!controlling) return
    const now = Date.now()
    if (now - lastMove.current < 33) return
    lastMove.current = now
    const { x, y } = point(event)
    send({ kind: 'mouseMove', x, y, buttons: event.buttons })
  }
  const onWheel = (event: WheelEvent<HTMLDivElement>) => {
    if (!controlling) return
    const { x, y } = point(event)
    send({ kind: 'wheel', x, y, deltaX: event.deltaX, deltaY: event.deltaY })
  }
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (!controlling) return
    const label = keyLabel(event)
    if (!label) return
    event.preventDefault()
    send({ kind: 'key', key: label })
  }

  return (
    <Sheet open={open} onClose={close} width={960} height={720}>
      <SheetPage className="live-tab-page">
        <ModalHeader
          icon="globe"
          title={tr('Live tab')}
          subtitle={tr("Watch this bot's browser tab and take control with your pointer and keyboard.")}
          onClose={close}
          trailing={controlling ? <span className="live-tab-badge"><span className="live-tab-dot" />{tr('Controlling')}</span> : null}
        />
        <HStack gap={8} className="live-tab-bar">
          <Button onClick={() => void toggle()} disabled={busy || !frame?.image}>{controlling ? tr('Release') : tr('Take control')}</Button>
          <Text size="caption" tone="secondary" truncate title={frame?.url ?? ''}>{frame?.title || tr('Loading the tab…')}</Text>
        </HStack>
        {error ? <Text role="alert" tone="danger" className="live-tab-error">{error}</Text> : null}
        <div
          className={`live-tab-stage${controlling ? ' controlling' : ''}`}
          tabIndex={0}
          onPointerDown={onPointerDown}
          onPointerUp={onPointerUp}
          onPointerMove={onPointerMove}
          onWheel={onWheel}
          onKeyDown={onKeyDown}
        >
          {frame?.image ? (
            <img ref={image} src={frame.image} alt={tr("The bot's browser tab")} draggable={false} />
          ) : (
            <HStack gap={8} className="live-tab-waiting" justify="center">
              <Spinner size={18} />
              <Text tone="secondary">{tr('Waiting for the first frame…')}</Text>
            </HStack>
          )}
        </div>
        <Text size="caption" tone="tertiary" className="live-tab-foot">
          {tr('The page is captured at most twice a second. Closing this sheet stops the stream and releases control.')}
        </Text>
      </SheetPage>
    </Sheet>
  )
}
