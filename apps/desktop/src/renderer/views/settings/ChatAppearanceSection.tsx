import { CHAT_BACKGROUNDS, CHAT_COLOURS, type ChatPreset, chatChoiceId, decodeChatChoice, encodeChatChoice } from '../../lib/chat-appearance'
import { CardSection, ValueRow } from '../../components/Controls'
import { HStack } from '../../components/Stack'
import { prefs, usePref } from '../../lib/prefs'
import { useDark } from '../../lib/theme'
import { useTr } from '../../lib/use-language'
import './chat-appearance.css'

/**
 * Settings → General → Chat: the colour of the person's own bubbles and the colour behind the chat.
 * Each row edits the theme the window shows; the other theme keeps its own choice.
 */
export function ChatAppearanceSection() {
  const tr = useTr()
  const dark = useDark()
  const [colour, setColour] = usePref(prefs.chatColour)
  const [background, setBackground] = usePref(prefs.chatBackground)
  return (
    <CardSection title="Chat">
      <SwatchRow
        label="Chat colour"
        detail="The colour of your own messages, for each theme."
        presets={CHAT_COLOURS}
        selected={chatChoiceId(decodeChatChoice(colour, CHAT_COLOURS.map((preset) => preset.id)), dark)}
        dark={dark}
        onSelect={(id) => setColour(encodeChatChoice(withChoice(colour, CHAT_COLOURS, id, dark)))}
      />
      <SwatchRow
        label="Chat background"
        detail="The colour behind the chat, for each theme."
        presets={CHAT_BACKGROUNDS}
        selected={chatChoiceId(decodeChatChoice(background, CHAT_BACKGROUNDS.map((preset) => preset.id)), dark)}
        dark={dark}
        onSelect={(id) => setBackground(encodeChatChoice(withChoice(background, CHAT_BACKGROUNDS, id, dark)))}
      />
    </CardSection>
  )
}

/** The saved choice with one theme's preset replaced; the other theme's choice stays. */
function withChoice(stored: string, presets: readonly ChatPreset[], id: string | null, dark: boolean) {
  const choice = decodeChatChoice(stored, presets.map((preset) => preset.id))
  return dark ? { ...choice, dark: id } : { ...choice, light: id }
}

function SwatchRow({ label, detail, presets, selected, dark, onSelect }: {
  label: string
  detail: string
  presets: readonly ChatPreset[]
  selected: string | null
  dark: boolean
  onSelect: (id: string | null) => void
}) {
  const tr = useTr()
  return (
    <ValueRow label={label} detail={`${tr(detail)} · ${tr(dark ? 'Dark' : 'Light')}`}>
      <HStack gap={6} className="chat-swatches">
        <button
          type="button"
          className={`chat-swatch chat-swatch-none${selected === null ? ' is-selected' : ''}`}
          aria-label={tr('None')}
          aria-pressed={selected === null}
          onClick={() => onSelect(null)}
        />
        {presets.map((preset) => (
          <button
            key={preset.id}
            type="button"
            className={`chat-swatch${selected === preset.id ? ' is-selected' : ''}`}
            style={{ background: dark ? preset.dark : preset.light }}
            aria-label={tr(preset.id.charAt(0).toUpperCase() + preset.id.slice(1))}
            aria-pressed={selected === preset.id}
            onClick={() => onSelect(preset.id)}
          />
        ))}
      </HStack>
    </ValueRow>
  )
}
