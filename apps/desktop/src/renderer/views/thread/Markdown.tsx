import { createContext, memo, useContext, useState, type MouseEvent, type ReactNode } from 'react'
import { math_blocks, parseMarkdown, type Block } from './markdown-blocks.ts'
import { Hairline } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { highlightSpans } from './code-highlight.ts'
import { HScroll } from './HScroll.tsx'
import { looksLikeDiff, parseDiff, type DiffRow } from './diff-rows.ts'
import { withChatLinkSource } from './link-source.ts'
import { chatFileLink, file_link_target, isMissingFileCandidate, linkTarget, previewKind } from '../../lib/chat-file-link.ts'
import { LocalFilePreviewSheet, LocalPreviewContext, type LocalPreview } from './PagePreview.tsx'
import './markdown.css'

// The Mac's Markdown for messages: the blocks from `markdown-blocks.ts`, with inline styling like
// `AttributedString(markdown:)`.

/**
 * The file paths this chat has already seen, used to resolve a written path with folders
 * (OpenBot 0.78.0). Without them a bare name still matches by name and a folder chain matches
 * nothing, so the link handler reports a missing file instead of opening the wrong one.
 */
export const FileCandidatesContext = createContext<readonly string[]>([])

/**
 * A link in a reply: PDF and HTML paths open in the chat's inert preview, other file types and web
 * URLs keep opening in the computer's own app. A full path in a code span (including .docx /
 * .pages / .rtf) is a file, while commands stay plain code (OpenBot nightly triage-12380).
 */
function FileAnchor({ path, href, label, line, column }: { path: string; href: string; label: ReactNode; line?: number; column?: number }) {
  const preview = useContext(LocalPreviewContext)
  const knownPaths = useContext(FileCandidatesContext)
  const open = (e: MouseEvent) => {
    e.preventDefault()
    // Only a written path that still resolves can preview; a bare name needs a known file.
    const kind = chatFileLink(path, knownPaths) ? previewKind(path) : null
    if (kind) preview({ path, href, kind, line, column })
    else window.codync.app.openExternal(href)
  }
  const target = line ? `${path}:${line}${column ? `:${column}` : ''}` : path
  return <a href={href} title={target} className="md-link" onClick={open}>{label}</a>
}

/** Inline Markdown: `code` (tinted), **bold**, *italic*, ~~strike~~, [links](url). */
export function inline(text: string, key = 'i', knownPaths: readonly string[] = []): ReactNode[] {
  const out: ReactNode[] = []
  // Code spans first: nothing inside them is styled, except a full path to a file.
  text.split(/(`[^`\n]+`)/).forEach((part, i) => {
    if (part.length > 2 && part.startsWith('`') && part.endsWith('`')) {
      const code = part.slice(1, -1)
      const file = chatFileLink(code, knownPaths)
      out.push(file
        ? <FileAnchor key={`${key}${i}`} path={file.path} href={file.href} label={code} />
        : <code key={`${key}${i}`} className="md-code-span">{code}</code>)
    } else if (part) out.push(...withMath(part, `${key}${i}-`, knownPaths))
  })
  return out
}

/** A formula is escaped text in a CSS class: no KaTeX, no HTML injection and no network lookup. */
function withMath(text: string, key: string, knownPaths: readonly string[]): ReactNode[] {
  const out: ReactNode[] = []
  math_blocks(text).forEach((part, i) => {
    const k = `${key}${i}`
    if (part.kind === 'text') out.push(...emphasis(part.text, `${k}-`, knownPaths))
    else out.push(
      <span key={k} className={part.kind === 'block' ? 'md-math md-math-block' : 'md-math'} role="math">{part.text}</span>,
    )
  })
  return out
}

// A link target inside `<…>` may hold spaces; CommonMark ends a bare target at the first space.
const pattern = /\[([^\]]+)\]\((?:<[^>]*>|[^)\s]+)\)|\*\*([^*]+)\*\*|__([^_]+)__|~~([^~]+)~~|\*([^*\n]+)\*|(?<![\w])_([^_\n]+)_(?![\w])/
function emphasis(text: string, key: string, knownPaths: readonly string[]): ReactNode[] {
  const out: ReactNode[] = []
  let rest = text
  let n = 0
  while (rest) {
    const m = pattern.exec(rest)
    if (!m) {
      out.push(rest)
      break
    }
    if (m.index > 0) out.push(rest.slice(0, m.index))
    const k = `${key}${n++}`
    if (m[1] !== undefined) {
      const written = linkTarget(m[2]!)
      const target = file_link_target(written)
      if (!target) {
        out.push(written)
        rest = rest.slice(m.index + m[0].length)
        continue
      }
      const file = chatFileLink(target.path, knownPaths)
      const label = emphasis(m[1], `${k}-`, knownPaths)
      if (file) {
        out.push(<FileAnchor key={k} path={file.path} href={file.href} label={label} line={target.line} column={target.column} />)
      } else if (isMissingFileCandidate(target.path)) {
        out.push(<span key={k} title={written}>{label}<span className="md-missing-note"> · File not found</span></span>)
      } else {
        out.push(<FileAnchor key={k} path={written} href={withChatLinkSource(written)} label={label} />)
      }
    } else if (m[3] !== undefined || m[4] !== undefined) out.push(<strong key={k} className="md-strong">{emphasis(m[3] ?? m[4]!, `${k}-`, knownPaths)}</strong>)
    else if (m[5] !== undefined) out.push(<s key={k}>{emphasis(m[5], `${k}-`, knownPaths)}</s>)
    else out.push(<em key={k}>{emphasis(m[6] ?? m[7]!, `${k}-`, knownPaths)}</em>)
    rest = rest.slice(m.index + m[0].length)
  }
  return out
}

const MarkdownBlock = memo(function MarkdownBlock({ block, knownPaths }: { block: Block; knownPaths: readonly string[] }) {
  switch (block.kind) {
    case 'paragraph':
      return <div className="md-text md-spaced md-body" dir="auto">{inline(block.text, 'i', knownPaths)}</div>
    case 'heading':
      return (
        <div className={`md-text md-spaced md-heading-${Math.min(block.level, 3)}`} dir="auto">
          {inline(block.text, 'i', knownPaths)}
        </div>
      )
    case 'bullet':
      return (
        <HStack align="baseline" gap={6} className="md-spaced md-body" style={{ paddingLeft: block.depth * 16 }}>
          <Text tone="secondary" className="md-marker">{block.marker}</Text>
          <span className="md-text md-bullet-text" dir="auto">{inline(block.text, 'i', knownPaths)}</span>
        </HStack>
      )
    case 'task':
      // A read-only marker: a reply's checklist is what the agent wrote, not a setting to change.
      return (
        <HStack align="baseline" gap={6} className="md-spaced md-body md-task" style={{ paddingLeft: block.depth * 16 }}>
          <span className={`md-task-box${block.checked ? ' checked' : ''}`} role="checkbox" aria-checked={block.checked}
            aria-readonly aria-label={block.checked ? 'Checked' : 'Unchecked'}>
            {block.checked ? <Icon name="checkmark" size={10} weight="bold" /> : null}
          </span>
          <span className={`md-text md-bullet-text${block.checked ? ' done' : ''}`} dir="auto">{inline(block.text, 'i', knownPaths)}</span>
        </HStack>
      )
    case 'rule':
      return <Hairline className="md-rule" />
    case 'table': {
      const columns = Math.max(block.header.length, ...block.rows.map((r) => r.length))
      const cols = Array.from({ length: columns }, (_, i) => i)
      return (
        <HScroll>
          <div className="md-table md-body" style={{ gridTemplateColumns: `repeat(${columns}, max-content)` }}>
            {cols.map((i) => (
              <Text key={`h${i}`} weight="semibold" dir="auto">{inline(block.header[i] ?? '', `h${i}`, knownPaths)}</Text>
            ))}
            {block.rows.map((row, r) => [
              <Hairline key={`l${r}`} style={{ gridColumn: `1 / span ${columns}` }} />,
              ...cols.map((i) => <span key={`${r}-${i}`} dir="auto">{inline(row[i] ?? '', `${r}-${i}`, knownPaths)}</span>),
            ])}
          </div>
        </HScroll>
      )
    }
    case 'quote':
      return <div className="md-text md-spaced md-body md-quote" dir="auto">{inline(block.text, 'i', knownPaths)}</div>
    case 'math':
      return <div className="md-math md-math-block md-spaced" dir="auto" role="math">{block.text}</div>
    case 'code':
      return <CodeBlock text={block.text} language={block.language} />
  }
}, (a, b) => JSON.stringify(a.block) === JSON.stringify(b.block))

/**
 * A fenced code block of a reply (idea 25): the language, a per-block wrap switch and a copy
 * button above the code. Copy puts the literal text on the clipboard, spaces and line breaks
 * intact. Diff blocks render through [DiffBlock]; other languages are highlighted locally.
 */
function CodeBlock({ text, language }: { text: string; language: string }) {
  const diff = looksLikeDiff(language, text) ? parseDiff(text) : null
  if (diff) return <DiffBlock rows={diff} original={text} language={language} />
  return <CodeFrame label={language.trim() || 'Code'} copyText={text} />
}

function CodeFrame({ label, copyText }: { label: string; copyText: string }) {
  const [wrap, setWrap] = useState(false)
  const [copied, setCopied] = useState(false)
  const copy = () => {
    void window.codync.app.copy(copyText)
    setCopied(true)
    window.setTimeout(() => setCopied(false), 1500)
  }
  return (
    <div className="md-code">
      <div className="md-code-header">
        <Text size="caption" mono tone="secondary" className="md-code-label">{label}</Text>
        <button className="md-code-action" role="switch" aria-checked={wrap} aria-label="Wrap lines" title="Wrap lines"
          onClick={() => setWrap((w) => !w)}>
          <span className={wrap ? 'md-code-action-text on' : 'md-code-action-text'}>Wrap</span>
        </button>
        <button className="md-code-action" aria-label={copied ? 'Copied' : 'Copy code'} title={copied ? 'Copied' : 'Copy code'} onClick={copy}>
          <Icon name={copied ? 'checkmark' : 'doc.on.doc'} size={12} weight="medium" />
        </button>
      </div>
      <HScroll><pre className={wrap ? 'md-code-text wrap' : 'md-code-text'}>{highlighted(copyText, label)}</pre></HScroll>
    </div>
  )
}

/** A unified diff with old and new line numbers where the hunk ranges allow (idea 26). */
function DiffBlock({ rows, original, language }: { rows: DiffRow[]; original: string; language: string }) {
  const [copied, setCopied] = useState(false)
  const copy = () => {
    void window.codync.app.copy(original)
    setCopied(true)
    window.setTimeout(() => setCopied(false), 1500)
  }
  return (
    <div className="md-code">
      <div className="md-code-header">
        <Text size="caption" mono tone="secondary" className="md-code-label">{language.trim() || 'diff'}</Text>
        <button className="md-code-action" aria-label={copied ? 'Copied' : 'Copy code'} title={copied ? 'Copied' : 'Copy code'} onClick={copy}>
          <Icon name={copied ? 'checkmark' : 'doc.on.doc'} size={12} weight="medium" />
        </button>
      </div>
      <HScroll>
        <div className="md-code-text md-diff">
          {rows.map((row, i) => <DiffLine key={i} row={row} />)}
        </div>
      </HScroll>
    </div>
  )
}

function DiffLine({ row }: { row: DiffRow }) {
  const numbered = row.kind === 'context' || row.kind === 'added' || row.kind === 'removed'
  const tone = row.kind === 'added' ? 'added' : row.kind === 'removed' ? 'removed' : row.kind === 'header' || row.kind === 'hunk' || row.kind === 'note' ? 'secondary' : 'text'
  const tint = row.kind === 'added' ? 'added' : row.kind === 'removed' ? 'removed' : row.kind === 'hunk' ? 'accent' : null
  return (
    <div className={`md-diff-line ${tint ? `tint-${tint}` : ''}`}>
      {numbered ? (
        <>
          <span className="md-diff-num">{row.kind === 'removed' || row.kind === 'context' ? row.oldNumber ?? '' : ''}</span>
          <span className="md-diff-num">{row.kind === 'added' || row.kind === 'context' ? row.newNumber ?? '' : ''}</span>
        </>
      ) : null}
      <Text as="span" mono tone={tone} className="md-diff-text">{row.raw || ' '}</Text>
    </div>
  )
}

/** The code with local highlighting; unknown languages and huge blocks stay plain text. */
function highlighted(text: string, language: string): ReactNode {
  const spans = highlightSpans(text, language)
  if (!spans.length) return text
  const out: ReactNode[] = []
  let pos = 0
  spans.forEach((s, i) => {
    if (s.start > pos) out.push(text.slice(pos, s.start))
    out.push(<span key={i} className={`hl-${s.kind}`}>{text.slice(s.start, s.end)}</span>)
    pos = s.end
  })
  if (pos < text.length) out.push(text.slice(pos))
  return out
}

/** The desktop's Markdown for messages; a block that didn't change keeps its rendering. */
export const MarkdownText = memo(function MarkdownText({ text, knownFiles = [] }: { text: string; knownFiles?: readonly string[] }) {
  const blocks = parseMarkdown(text)
  const [local, setLocal] = useState<LocalPreview | null>(null)
  return (
    <LocalPreviewContext.Provider value={setLocal}>
      <FileCandidatesContext.Provider value={knownFiles}>
        <VStack gap={10} shrink className="markdown selectable">
          {blocks.map((b, i) => (
            <MarkdownBlock key={i} block={b} knownPaths={knownFiles} />
          ))}
        </VStack>
      </FileCandidatesContext.Provider>
      <LocalFilePreviewSheet preview={local} close={() => setLocal(null)} />
    </LocalPreviewContext.Provider>
  )
})
