import { useEffect, useRef, useState, type KeyboardEvent } from 'react'
import { Button, CardForm, CardSection, IconButton, Reveal } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import {
  captureChord, chordLabel, KEYBOARD_SHORTCUTS, REBINDABLE_SHORTCUTS, resolveBindings,
  type Chord, type Shortcut,
} from '../../components/keyboard-shortcuts'
import { prefs, usePref } from '../../lib/prefs'
import { useTr } from '../../lib/use-language'
import './keyboard-settings.css'

/** Why a recording was refused, in the row's own words (Escape just ends the recording). */
const refusal: Record<string, string> = {
  modifier: 'Press a key as well as the modifier, not the modifier on its own.',
  reserved: 'That key is reserved for the app.',
  duplicate: 'That shortcut is already used by another row.',
  empty: 'Press a key combination.',
}

/** Settings › Keyboard: one chord per shortcut, captured on this device only. */
export function KeyboardSettings() {
  const tr = useTr()
  const [bindings, setBindings] = usePref(prefs.keyboardBindings)
  const [capturing, setCapturing] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const groups = [...new Set(KEYBOARD_SHORTCUTS.map((shortcut) => shortcut.group))]

  const reset = (id: string) => {
    const next = { ...bindings }
    delete next[id]
    setBindings(resolveBindings(next))
    if (capturing === id) setCapturing(null)
    setError(null)
  }
  const resetAll = () => {
    setBindings(resolveBindings({}))
    setCapturing(null)
    setError(null)
  }
  const record = (id: string, event: KeyboardEvent) => {
    event.preventDefault()
    event.stopPropagation()
    // A built-in chord stays claimed even when its row has moved: resolveBindings refuses it too.
    const taken = REBINDABLE_SHORTCUTS.filter((row) => row.id !== id).flatMap((row) => [row.chord!, bindings[row.id] ?? row.chord!])
    const result = captureChord(event.nativeEvent, taken)
    if (!result.ok) {
      if (result.reason === 'cancelled') {
        setCapturing(null)
        setError(null)
      } else {
        setError(refusal[result.reason])
      }
      return
    }
    setBindings(resolveBindings({ ...bindings, [id]: result.chord }))
    setCapturing(null)
    setError(null)
  }

  return (
    <CardForm>
      <VStack gap={12}>
        <HStack gap={8}>
          <Text tone="secondary" grow>{tr('Change a shortcut, then press the new keys. Escape cancels. The changes apply to this device only.')}</Text>
          <Button kind="secondary" onClick={resetAll}>{tr('Reset all')}</Button>
        </HStack>
        <Reveal show={error !== null}>
          <Text as="div" size="footnote" tone="attention">{error ? tr(error) : ''}</Text>
        </Reveal>
        {groups.map((group) => (
          <CardSection key={group} title={group}>
            {KEYBOARD_SHORTCUTS.filter((shortcut) => shortcut.group === group).map((shortcut) => (
              <ShortcutRow
                key={shortcut.id}
                shortcut={shortcut}
                chord={shortcut.rebindable ? bindings[shortcut.id] ?? shortcut.chord : undefined}
                capturing={capturing === shortcut.id}
                onStart={() => { setCapturing(shortcut.id); setError(null) }}
                onCancel={() => { if (capturing === shortcut.id) { setCapturing(null); setError(null) } }}
                onRecord={(event) => record(shortcut.id, event)}
                onReset={() => reset(shortcut.id)}
              />
            ))}
          </CardSection>
        ))}
      </VStack>
    </CardForm>
  )
}

function ShortcutRow({ shortcut, chord, capturing, onStart, onCancel, onRecord, onReset }: {
  shortcut: Shortcut
  chord?: Chord
  capturing: boolean
  onStart: () => void
  onCancel: () => void
  onRecord: (event: KeyboardEvent) => void
  onReset: () => void
}) {
  const tr = useTr()
  const recorder = useRef<HTMLDivElement>(null)
  // The recorder has to hold focus to hear the chord; leaving it ends the recording.
  useEffect(() => { if (capturing) recorder.current?.focus() }, [capturing])
  const fixed = !shortcut.rebindable || !shortcut.chord
  const label = chord ? chordLabel(chord, window.codync.platform) : shortcut.keys
  return (
    <VStack gap={capturing ? 6 : 0}>
      <HStack gap={8}>
        <Text tone="text" grow>{tr(shortcut.label)}</Text>
        {fixed ? (
          <span className="keyboard-fixed-keys" aria-label={label}>{label}</span>
        ) : (
          <button className={`keyboard-chord${capturing ? ' capturing' : ''}`} title={tr('Change')}
            aria-label={`${tr(shortcut.label)}, ${label}. ${tr('Change')}`} onClick={onStart}>
            {capturing ? tr('Press keys…') : label}
          </button>
        )}
        {fixed ? null : <IconButton title="Reset" icon="arrow.counterclockwise" onClick={onReset} />}
      </HStack>
      <Reveal show={capturing}>
        <div ref={recorder} tabIndex={0} role="button" aria-label={tr('Recording shortcut')}
          className="settings-talk-recorder" onKeyDown={onRecord} onBlur={onCancel}>
          <Text size="footnote" tone="secondary">{tr('Press the new keys, or Escape to cancel.')}</Text>
        </div>
      </Reveal>
    </VStack>
  )
}
