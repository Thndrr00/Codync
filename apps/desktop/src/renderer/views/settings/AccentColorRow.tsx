import { useEffect, useState } from 'react'
import { Button } from '../../components/Controls'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { AVATAR_COLORS } from '../../lib/theme'
import { normalizeHex } from '../../lib/settings-io'
import { usePref, prefs } from '../../lib/prefs'
import { useTr } from '../../lib/use-language'

/**
 * Settings › Appearance: the curated swatches stay as they are, and a hex field lets the person pick
 * any colour. Only a valid `#RGB`/`#RRGGBB` reaches the accent pref (and the CSS variables behind it).
 */
export function AccentColorRow() {
  const tr = useTr()
  const [value, setValue] = usePref(prefs.accentColor)
  const [draft, setDraft] = useState(value ?? '')
  useEffect(() => setDraft(value ?? ''), [value])
  const valid = normalizeHex(draft)
  const invalid = draft.trim().length > 0 && !valid

  const choose = (hex: string | null) => {
    setValue(hex)
    setDraft(hex ?? '')
  }
  const commit = () => {
    if (valid) choose(valid)
  }

  return (
    <VStack gap={8} className="accent-picker">
      <Text size="callout" weight="medium">{tr('Accent color')}</Text>
      <HStack gap={6} wrap className="accent-swatches">
        <button
          type="button"
          className={`accent-swatch default${value === null ? ' chosen' : ''}`}
          aria-label={tr('Default accent')}
          title={tr('Default accent')}
          aria-pressed={value === null}
          onClick={() => choose(null)}
        />
        {AVATAR_COLORS.map((swatch) => (
          <button
            key={swatch.id}
            type="button"
            className={`accent-swatch${value === swatch.hex ? ' chosen' : ''}`}
            style={{ background: swatch.hex }}
            aria-label={tr('{name} accent').replace('{name}', swatch.label)}
            title={swatch.label}
            aria-pressed={value === swatch.hex}
            onClick={() => choose(swatch.hex)}
          />
        ))}
      </HStack>
      <HStack gap={6}>
        <input
          className="accent-hex-field"
          aria-label={tr('Custom accent hex')}
          placeholder="#1084FE"
          spellCheck={false}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => { if (e.key === 'Enter') commit() }}
          onBlur={commit}
        />
        <Button kind="secondary" onClick={commit} disabled={!valid || valid === value}>{tr('Use')}</Button>
      </HStack>
      {invalid ? <Text size="caption" tone="danger" role="alert">{tr('Enter a hex color such as #1084FE.')}</Text> : null}
    </VStack>
  )
}
