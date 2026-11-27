// Block-level Markdown as replies use it (kit MarkdownBlocks): paragraphs, headings, nested
// lists, quotes, rules, tables and fenced code. Pure parsing, tested on its own.

export type Block =
  | { kind: 'paragraph'; text: string }
  | { kind: 'heading'; text: string; level: number }
  | { kind: 'bullet'; text: string; marker: string; depth: number }
  | { kind: 'task'; text: string; checked: boolean; depth: number }
  | { kind: 'quote'; text: string }
  | { kind: 'code'; text: string; language: string }
  | { kind: 'table'; header: string[]; rows: string[][] }
  | { kind: 'math'; text: string }
  | { kind: 'rule' }

/** One piece of a reply after formulas have been taken out: literal text or escaped math source. */
export type MathPart =
  | { kind: 'text'; text: string }
  | { kind: 'inline'; text: string }
  | { kind: 'block'; text: string }

/** A bound so a stray dollar sign cannot swallow an entire reply. */
const MATH_LIMIT = 2_000

/**
 * Pure: splits text into literal text and `$…$` / `$$…$$` formulas. A missing close, empty content,
 * a newline inside an inline formula, or a dollar sign inside one stays literal (so prices and sums
 * keep their words). Only source characters are returned: the renderer escapes them in a CSS class,
 * with no HTML, script, dependency or network involved (OpenBot 0.70.0 math in chat).
 */
export function math_blocks(text: string): MathPart[] {
  const parts: MathPart[] = []
  const literal = (value: string) => {
    if (!value) return
    const previous = parts[parts.length - 1]
    if (previous?.kind === 'text') parts[parts.length - 1] = { kind: 'text', text: previous.text + value }
    else parts.push({ kind: 'text', text: value })
  }
  let rest = text
  while (rest) {
    const blockAt = rest.indexOf('$$')
    const dollarAt = rest.indexOf('$')
    if (dollarAt < 0) {
      literal(rest)
      break
    }
    const block = blockAt === dollarAt
    const delimiter = block ? '$$' : '$'
    const after = rest.slice(dollarAt + delimiter.length)
    const close = after.indexOf(delimiter)
    const content = close < 0 ? '' : after.slice(0, close)
    let valid = close >= 0 && !!content.trim() && [...content].length <= MATH_LIMIT
    // An inline formula holds no other dollar or newline; a price like `$5 and $10` has trailing
    // space in its "content" and stays text.
    if (valid && !block) valid = !/[$\r\n]/.test(content) && !/^\s|\s$/.test(content)
    if (!valid) {
      literal(rest.slice(0, dollarAt + delimiter.length))
      rest = rest.slice(dollarAt + delimiter.length)
      continue
    }
    literal(rest.slice(0, dollarAt))
    parts.push({ kind: block ? 'block' : 'inline', text: content.trim() })
    rest = after.slice(close + delimiter.length)
  }
  return parts
}

const isTableRule = (line: string) => line.includes('-') && [...line].every((c) => '|-: '.includes(c))
const cells = (line: string) => {
  let row = line
  if (row.startsWith('|')) row = row.slice(1)
  if (row.endsWith('|')) row = row.slice(0, -1)
  return row.split('|').map((c) => c.trim())
}

export function parseMarkdown(source: string): Block[] {
  const blocks: Block[] = []
  const lines = source.split('\n')
  let paragraph: string[] = []
  let table: string[] = []
  let code: string[] | null = null
  let language = ''
  const flush = () => {
    if (paragraph.length) {
      blocks.push({ kind: 'paragraph', text: paragraph.join('\n') })
      paragraph = []
    }
    if (table.length) {
      // A header and its `|---|` line make a table; anything less stays text.
      if (table.length >= 2 && isTableRule(table[1]!)) blocks.push({ kind: 'table', header: cells(table[0]!), rows: table.slice(2).map(cells) })
      else blocks.push({ kind: 'paragraph', text: table.join('\n') })
      table = []
    }
  }
  for (let i = 0; i < lines.length; i++) {
    const raw = lines[i]!
    const line = raw.trim()
    if (line.startsWith('```')) {
      if (code) {
        blocks.push({ kind: 'code', text: code.join('\n'), language })
        code = null
      } else {
        flush()
        code = []
        language = line.slice(3)
      }
      continue
    }
    if (code) {
      code.push(raw)
      continue
    }
    // A `$$…$$` paragraph is a formula block, even across lines. An unclosed or mixed one falls
    // through as ordinary text; `math_blocks` in Markdown.tsx still handles inline `$…$`.
    if (line.startsWith('$$')) {
      const single = math_blocks(line)
      let collected = line
      let end = i
      if (!(single.length === 1 && single[0]!.kind === 'block')) {
        for (let j = i + 1; j < lines.length; j++) {
          const next = lines[j]!.trim()
          collected += `\n${next}`
          end = j
          if (next.includes('$$')) break
        }
      }
      const parts = math_blocks(collected)
      if (parts.length === 1 && parts[0]!.kind === 'block') {
        flush()
        blocks.push({ kind: 'math', text: parts[0]!.text })
        i = end
        continue
      }
    }
    if (line.startsWith('|')) {
      if (paragraph.length) flush()
      table.push(line)
      continue
    }
    if (table.length) flush()
    const depth = Math.min(Math.floor((/^[ \t]*/.exec(raw)?.[0].length ?? 0) / 2), 3)
    const heading = /^(#+) (.*)$/.exec(line)
    const ordered = /^(\d+\.) (.*)$/.exec(line)
    if (!line) flush()
    else if (line === '---' || line === '***' || line === '___') {
      flush()
      blocks.push({ kind: 'rule' })
    } else if (heading) {
      flush()
      blocks.push({ kind: 'heading', text: heading[2]!.trim(), level: heading[1]!.length })
    } else if (/^[-*+•] /.test(line)) {
      flush()
      const content = line.slice(2)
      // A checkbox is a task even when the task tools wrote it as plain Markdown (OpenBot 0.74.0).
      const task = /^\[([ xX])\]\s+(.*)$/.exec(content)
      if (task) blocks.push({ kind: 'task', text: task[2]!.trim(), checked: task[1]!.toLowerCase() === 'x', depth })
      else blocks.push({ kind: 'bullet', text: content, marker: depth > 0 ? '◦' : '•', depth })
    } else if (ordered) {
      flush()
      blocks.push({ kind: 'bullet', text: ordered[2]!, marker: ordered[1]!, depth })
    } else if (line.startsWith('>')) {
      flush()
      blocks.push({ kind: 'quote', text: line.slice(1).trim() })
    } else paragraph.push(raw)
  }
  if (code) blocks.push({ kind: 'code', text: (code as string[]).join('\n'), language })
  flush()
  return blocks
}
