import { useState, type MouseEvent } from 'react'
import { Button } from './Controls'
import { Icon } from './Icon'
import { Sheet } from './Overlay'
import { ShimmerText } from './ShimmerText'
import { HStack, Spacer, VStack } from './Stack'
import { Text } from './Text'
import { useTr } from '../lib/use-language'

export interface ComputerFrame {
  url?: string
  title?: string
  action?: string
  screenshot?: string
  active?: boolean
}

/** Ideas 19 & 20: Live browser/computer activity card with interactive TakeoverModal. */
export function ComputerCard({
  frame,
  onClickPoint,
  onTypeText,
  onNavigate,
  onTakeOver,
  onFinishTakeover,
}: {
  frame: ComputerFrame
  onClickPoint?: (x: number, y: number) => void
  onTypeText?: (text: string) => void
  onNavigate?: (url: string) => void
  /** The person is taking it: the agent waits from here. */
  onTakeOver?: () => void
  /** Done: handed back, the agent goes on. */
  onFinishTakeover?: () => void
}) {
  const tr = useTr()
  const [takeoverOpen, setTakeoverOpen] = useState(false)
  const [typedText, setTypedText] = useState('')
  const [urlInput, setUrlInput] = useState(frame.url ?? '')

  const handleImageClick = (e: MouseEvent<HTMLImageElement>) => {
    if (!onClickPoint) return
    const rect = e.currentTarget.getBoundingClientRect()
    const x = Math.round(((e.clientX - rect.left) / rect.width) * 1280)
    const y = Math.round(((e.clientY - rect.top) / rect.height) * 800)
    onClickPoint(x, y)
  }

  return (
    <>
      <div
        className="card-section-rows"
        style={{
          padding: 12,
          borderRadius: 'var(--radius-l)',
          background: 'var(--group)',
        }}
      >
        <VStack gap={8}>
          <HStack align="center" gap={8}>
            <Icon name="globe" size={13} color="var(--secondary)" />
            <Text size="callout" weight="semibold" truncate>
              {frame.title || frame.url || 'Browser session'}
            </Text>
            <Spacer />
            {frame.active ? (
              <ShimmerText active>{frame.action || 'Browsing…'}</ShimmerText>
            ) : (
              <Text size="caption" tone="secondary">
                {frame.action || 'Idle'}
              </Text>
            )}
            {onTakeOver ? (
              <Button
                kind="secondary"
                onClick={() => {
                  onTakeOver()
                  setTakeoverOpen(true)
                }}
              >
                {tr('Take over')}
              </Button>
            ) : null}
            {onFinishTakeover ? (
              <Button
                kind="primary"
                onClick={() => {
                  setTakeoverOpen(false)
                  onFinishTakeover()
                }}
              >
                {tr('Done, hand back')}
              </Button>
            ) : null}
          </HStack>
          {frame.screenshot ? (
            <img
              src={frame.screenshot}
              alt={frame.title || 'Browser view'}
              onClick={() => setTakeoverOpen(true)}
              style={{
                width: '100%',
                maxHeight: 240,
                objectFit: 'cover',
                borderRadius: 'var(--radius-s)',
                cursor: 'pointer',
              }}
            />
          ) : null}
        </VStack>
      </div>

      <Sheet
        open={takeoverOpen}
        onClose={() => setTakeoverOpen(false)}
        width="min(920px, 92vw)"
      >
        <VStack gap={12} style={{ padding: 18 }}>
          <HStack align="center" gap={10}>
            <Icon name="hand.raised.fill" size={14} color="var(--warning)" />
            <Text size="title3" weight="semibold">
              {tr('Interactive Browser Takeover')}
            </Text>
            <Spacer />
            <Button
              kind="primary"
              onClick={() => {
                setTakeoverOpen(false)
                onFinishTakeover?.()
              }}
            >
              {tr('Done, hand back')}
            </Button>
          </HStack>

          {onNavigate ? (
            <HStack gap={8} align="center">
              <input
                value={urlInput}
                onChange={(e) => setUrlInput(e.target.value)}
                placeholder="https://…"
                style={{
                  flex: 1,
                  padding: '6px 10px',
                  borderRadius: 'var(--radius-s)',
                  background: 'var(--field)',
                }}
              />
              <Button kind="secondary" onClick={() => onNavigate(urlInput)}>
                Go
              </Button>
            </HStack>
          ) : null}

          {frame.screenshot ? (
            <div style={{ position: 'relative', overflow: 'hidden', borderRadius: 'var(--radius-m)' }}>
              <img
                src={frame.screenshot}
                alt="Interactive remote browser"
                onClick={handleImageClick}
                style={{ width: '100%', display: 'block', cursor: 'crosshair' }}
              />
            </div>
          ) : (
            <div style={{ padding: 36, textAlign: 'center', background: 'var(--field)', borderRadius: 'var(--radius-m)' }}>
              <Text tone="secondary">Waiting for live browser frame…</Text>
            </div>
          )}

          {onTypeText ? (
            <HStack gap={8} align="center">
              <input
                value={typedText}
                onChange={(e) => setTypedText(e.target.value)}
                placeholder="Type text into focused element…"
                style={{
                  flex: 1,
                  padding: '6px 10px',
                  borderRadius: 'var(--radius-s)',
                  background: 'var(--field)',
                }}
              />
              <Button
                kind="secondary"
                onClick={() => {
                  if (!typedText) return
                  onTypeText(typedText)
                  setTypedText('')
                }}
              >
                Send keys
              </Button>
            </HStack>
          ) : null}
        </VStack>
      </Sheet>
    </>
  )
}
