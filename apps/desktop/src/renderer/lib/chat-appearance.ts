/**
 * The 11 chat colours and the calm chat backgrounds of OpenBot 0.65, one choice for the light theme
 * and one for the dark. The same list is in the phones and the terminal UI
 * (`host/src/tui/appearance.rs`).
 */

export interface ChatPreset {
  id: string
  /** The shade that reads on the light theme. */
  light: string
  /** The shade that reads on the dark theme. */
  dark: string
}

export const CHAT_COLOURS: readonly ChatPreset[] = [
  { id: 'red', light: '#E35652', dark: '#F8706D' },
  { id: 'orange', light: '#D86706', dark: '#ED8125' },
  { id: 'yellow', light: '#D39809', dark: '#E8B027' },
  { id: 'green', light: '#06A74E', dark: '#25BF69' },
  { id: 'cyan', light: '#0DA195', dark: '#2BB9AD' },
  { id: 'sky', light: '#069BC2', dark: '#24B2D8' },
  { id: 'blue', light: '#408DEA', dark: '#5CA5FE' },
  { id: 'indigo', light: '#7B7DEB', dark: '#9596FF' },
  { id: 'violet', light: '#A66BE0', dark: '#BD85F5' },
  { id: 'magenta', light: '#CA5BB6', dark: '#E076CD' },
  { id: 'pink', light: '#DD5487', dark: '#F26FA0' },
]

export const CHAT_BACKGROUNDS: readonly ChatPreset[] = [
  { id: 'red', light: '#FFEFED', dark: '#311B19' },
  { id: 'orange', light: '#FEF0E9', dark: '#2F1D12' },
  { id: 'yellow', light: '#FAF2E6', dark: '#2A200D' },
  { id: 'green', light: '#EBF7ED', dark: '#152619' },
  { id: 'cyan', light: '#E7F8F5', dark: '#0A2724' },
  { id: 'sky', light: '#E7F6FD', dark: '#0C252E' },
  { id: 'blue', light: '#ECF4FF', dark: '#162232' },
  { id: 'indigo', light: '#F1F2FF', dark: '#1E2032' },
  { id: 'violet', light: '#F6F1FE', dark: '#251D2F' },
  { id: 'magenta', light: '#FCEFF8', dark: '#2C1B28' },
  { id: 'pink', light: '#FFEFF3', dark: '#2F1A21' },
]

/** A saved chat colour or background: one per appearance, `null` for the theme's own. */
export interface ChatChoice {
  light: string | null
  dark: string | null
}

const ids = (presets: readonly ChatPreset[]) => presets.map((preset) => preset.id)

/**
 * `"light/dark"`, `-` for none. A single id saved before the themes split means both, so a colour
 * chosen earlier survives this change.
 */
export function decodeChatChoice(stored: string, known: readonly string[]): ChatChoice {
  const value = stored.trim()
  const [lightPart, darkPart] = value.includes('/') ? value.split('/', 2) : [value, value]
  const pick = (id: string | undefined) => {
    const trimmed = id?.trim() ?? ''
    return known.includes(trimmed) ? trimmed : null
  }
  return { light: pick(lightPart), dark: pick(darkPart) }
}

export function encodeChatChoice(choice: ChatChoice): string {
  return `${choice.light ?? '-'}/${choice.dark ?? '-'}`
}

export function chatChoiceId(choice: ChatChoice, dark: boolean): string | null {
  return dark ? choice.dark : choice.light
}

/** The next choice in the list: none first, then each preset, then none again. */
export function cycleChatChoice(choice: ChatChoice, dark: boolean, known: readonly string[]): ChatChoice {
  const current = chatChoiceId(choice, dark)
  const at = current === null ? -1 : known.indexOf(current)
  const next = at + 1 < known.length ? known[at + 1]! : null
  return dark ? { ...choice, dark: next } : { ...choice, light: next }
}

export function chatPreset(presets: readonly ChatPreset[], id: string | null): ChatPreset | undefined {
  return id === null ? undefined : presets.find((preset) => preset.id === id)
}

/** `#RRGGBB` moved `amount` of the way to `#RRGGBB`. */
export function mixHex(from: string, to: string, amount: number): string {
  const read = (hex: string) => [1, 3, 5].map((at) => parseInt(hex.slice(at, at + 2), 16))
  const [r1, g1, b1] = read(from)
  const [r2, g2, b2] = read(to)
  const t = Math.min(Math.max(amount, 0), 1)
  return `#${[r1! + (r2! - r1!) * t, g1! + (g2! - g1!) * t, b1! + (b2! - b1!) * t]
    .map((channel) => Math.round(channel).toString(16).padStart(2, '0'))
    .join('')}`
}

/** The chat colour of one scheme, as a preset id. */
export function savedChatColour(stored: string, dark: boolean): ChatPreset | undefined {
  return chatPreset(CHAT_COLOURS, chatChoiceId(decodeChatChoice(stored, ids(CHAT_COLOURS)), dark))
}

/** The chat background of one scheme, as a preset id. */
export function savedChatBackground(stored: string, dark: boolean): ChatPreset | undefined {
  return chatPreset(CHAT_BACKGROUNDS, chatChoiceId(decodeChatChoice(stored, ids(CHAT_BACKGROUNDS)), dark))
}
