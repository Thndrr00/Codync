import { useState } from 'react'
import type { Entry } from '@shared/models'
import { Button, Spinner } from './Controls'
import { ModalHeader, SheetPage } from './Overlay'
import { HStack, VStack } from './Stack'
import { Text } from './Text'
import { useTr } from '../lib/use-language'
import { useStore } from '../store/context'

/** A fix as a short label. No map dependency: the agent gets a place, never an embedded map. */
export function coordinatesLabel(latitude: number, longitude: number): string {
  return `${latitude.toFixed(5)}, ${longitude.toFixed(5)}`
}

/** Only offer the current position where the browser actually exposes it; elsewhere just type a place. */
export function canUseCurrentPosition(): boolean {
  return typeof navigator !== 'undefined' && typeof navigator.geolocation?.getCurrentPosition === 'function'
}

/** The small sheet behind a location card's "Share location" button (OpenBot nightly v0.77.0). */
export function LocationSheet({ entry, close }: { entry: Entry; close: () => void }) {
  const tr = useTr()
  const store = useStore()
  const [place, setPlace] = useState('')
  const [busy, setBusy] = useState(false)
  const [locating, setLocating] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const share = async (text: string) => {
    const value = text.trim()
    if (!value || busy) return
    setBusy(true)
    setError(null)
    try {
      const c = await store.ready()
      await c.call('shareLocation', { entryId: entry.id, text: value })
      close()
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason))
      setBusy(false)
    }
  }

  const useCurrent = () => {
    if (!canUseCurrentPosition()) return
    setError(null)
    setLocating(true)
    try {
      navigator.geolocation.getCurrentPosition(
        (position) => {
          setLocating(false)
          setPlace(coordinatesLabel(position.coords.latitude, position.coords.longitude))
        },
        () => {
          setLocating(false)
          setError(tr('Current position is unavailable. Type the place instead.'))
        },
        { timeout: 10_000, maximumAge: 60_000 },
      )
    } catch {
      // A browser may expose the API yet refuse to start it (no permission, insecure context).
      setLocating(false)
      setError(tr('Current position is unavailable. Type the place instead.'))
    }
  }

  return (
    <SheetPage>
      <ModalHeader icon="pin" title={tr('Share your location')} subtitle={entry.data.title ?? tr('The agent asked where you are.')} onClose={close} />
      <VStack gap={12} className="group-form">
        <Text size="caption" tone="secondary">{tr('Type a place or an address. Nothing is sent until you share it.')}</Text>
        <input
          className="field-box"
          data-autofocus
          value={place}
          maxLength={500}
          spellCheck={false}
          aria-label={tr('Place or address')}
          placeholder={tr('e.g. 1 Main Street, Springfield')}
          onChange={(e) => setPlace(e.target.value)}
          onKeyDown={(e) => { if (e.key === 'Enter' && !e.nativeEvent.isComposing) void share(place) }}
        />
        {canUseCurrentPosition() ? (
          <Button kind="secondary" disabled={locating || busy} onClick={useCurrent}>
            {locating ? tr('Finding your position…') : tr('Use current position')}
          </Button>
        ) : null}
        {error ? <Text role="alert" tone="danger">{error}</Text> : null}
        <HStack gap={8}>
          <Button kind="primary" disabled={!place.trim() || busy} onClick={() => void share(place)}>
            {busy ? <Spinner size={14} /> : tr('Share location')}
          </Button>
          <Button kind="secondary" onClick={close}>{tr('Cancel')}</Button>
        </HStack>
      </VStack>
    </SheetPage>
  )
}
