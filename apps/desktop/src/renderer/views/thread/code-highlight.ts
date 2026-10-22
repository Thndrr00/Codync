// Local syntax highlighting for fenced code (idea 25, as Android's CodeHighlight): a small scanner
// that marks comments, strings, numbers and keywords. Unknown languages and blocks past
// `MAX_CHARS` return no spans, so they render as plain text; nothing here reaches the network.

export type HighlightKind = 'keyword' | 'literal' | 'comment' | 'number'

export interface HighlightSpan {
  start: number
  end: number
  kind: HighlightKind
}

/** Longer blocks stay plain text: tokenizing them would stall a streaming reply. */
export const MAX_HIGHLIGHT_CHARS = 20_000

interface Family {
  lineComments: string[]
  blockComments: boolean
  backtickStrings: boolean
  keywords: Set<string>
}

const words = (list: string) => new Set(list.split(' '))

const cLike: Family = {
  lineComments: ['//'],
  blockComments: true,
  backtickStrings: true,
  keywords: words(
    'abstract as async await break case catch class const continue data default defer do else enum extends false final finally fn for fun func function if impl ' +
    'implements import in interface internal is let match mod mut new null nil object override package private protected pub public return self static struct super switch ' +
    'this throw throws trait true try type typealias use val var void when where while yield',
  ),
}
const hash: Family = {
  lineComments: ['#'],
  blockComments: false,
  backtickStrings: false,
  keywords: words(
    'and as async await break case class continue def del do done elif else end esac except export False false finally for from function if import in is ' +
    'lambda None nil not null or pass raise return then True true try while with yield',
  ),
}
const json: Family = { lineComments: [], blockComments: false, backtickStrings: false, keywords: words('true false null') }

const families = new Map<string, Family>([
  ...['kotlin', 'kt', 'java', 'swift', 'rust', 'rs', 'go', 'js', 'javascript', 'ts', 'typescript', 'tsx', 'jsx', 'c', 'cpp', 'c++', 'cs', 'csharp', 'dart'].map((l) => [l, cLike] as const),
  ...['python', 'py', 'sh', 'bash', 'zsh', 'shell', 'yaml', 'yml', 'toml', 'ruby', 'rb', 'dockerfile'].map((l) => [l, hash] as const),
  ...['json', 'jsonc'].map((l) => [l, json] as const),
])

const family = (language: string) => families.get(language.trim().split(' ')[0]!.toLowerCase())

const isDigit = (c: string) => /\p{Nd}/u.test(c)
const isLetter = (c: string) => /\p{L}/u.test(c)
const isLetterOrDigit = (c: string) => isLetter(c) || isDigit(c)

export const supportsHighlight = (language: string) => family(language) !== undefined

const lineEnd = (code: string, from: number) => {
  const at = code.indexOf('\n', from)
  return at < 0 ? code.length : at
}

/** The index after the string opened at `from`; an unclosed one ends at its line (or, for backticks, the text). */
function stringEnd(code: string, from: number, multiline: boolean): number {
  const quote = code[from]
  let i = from + 1
  while (i < code.length) {
    const c = code[i]
    if (c === '\\') i += 2
    else if (c === quote) return i + 1
    else if (c === '\n' && !multiline) return i
    else i++
  }
  return code.length
}

/** Ordered, non-overlapping spans of `code`; empty when the language is unknown or the block too long. */
export function highlightSpans(code: string, language: string): HighlightSpan[] {
  const fam = family(language)
  if (!fam || code.length > MAX_HIGHLIGHT_CHARS) return []
  const spans: HighlightSpan[] = []
  let i = 0
  while (i < code.length) {
    const c = code[i]!
    if (fam.lineComments.some((marker) => code.startsWith(marker, i))) {
      const end = lineEnd(code, i)
      spans.push({ start: i, end, kind: 'comment' })
      i = end
    } else if (fam.blockComments && code.startsWith('/*', i)) {
      const close = code.indexOf('*/', i + 2)
      const end = close < 0 ? code.length : close + 2
      spans.push({ start: i, end, kind: 'comment' })
      i = end
    } else if (c === '"' || c === "'" || (c === '`' && fam.backtickStrings)) {
      const end = stringEnd(code, i, c === '`')
      spans.push({ start: i, end, kind: 'literal' })
      i = end
    } else if (isDigit(c) && (i === 0 || !(isLetterOrDigit(code[i - 1]!) || code[i - 1] === '_'))) {
      let end = i + 1
      while (end < code.length && (isLetterOrDigit(code[end]!) || code[end] === '.' || code[end] === '_')) end++
      spans.push({ start: i, end, kind: 'number' })
      i = end
    } else if (isLetter(c) || c === '_') {
      let end = i + 1
      while (end < code.length && (isLetterOrDigit(code[end]!) || code[end] === '_')) end++
      if (fam.keywords.has(code.slice(i, end))) spans.push({ start: i, end, kind: 'keyword' })
      i = end
    } else i++
  }
  return spans
}
