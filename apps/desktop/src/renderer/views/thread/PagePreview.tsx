import { createContext, useMemo, useRef, useState } from 'react'
import { Button } from '../../components/Controls'
import { ModalHeader, Sheet, SheetPage } from '../../components/Overlay'
import { Text } from '../../components/Text'
import { fileReferenceName, hash_target, type PreviewKind } from '../../lib/chat-file-link.ts'
import { useTr } from '../../lib/use-language'
import { safePageHtml } from '../../lib/page-preview'
import { QUOTE_LIMIT, VISUAL_REPLY_LIMIT, quote_text, visual_reply } from '../../lib/visual-reply'
import './page-preview.css'

/**
 * An HTML artifact published above its agent's reply (OpenBot `html_render`). It reuses the inert
 * preview: the frame keeps an empty `sandbox` and the same sanitizer, so a visual reply cannot run a
 * script or fetch anything. `order: -1` places it ahead of the reply inside the message's column;
 * no new window, service or IPC is involved. Selecting text on the page (or its caption) makes a
 * bounded quote chip, mirroring `quote_text` in `lib/visual-reply.ts`.
 */
export function VisualReply({ html, title, height = 360, onQuote }: {
  html: string | null
  title: string
  height?: number
  onQuote?: (text: string) => void
}) {
  const tr = useTr()
  const frame = useRef<HTMLIFrameElement>(null)
  const [quote, setQuote] = useState('')
  const reply = useMemo(() => visual_reply(html, VISUAL_REPLY_LIMIT), [html])
  const srcDoc = useMemo(() => (reply ? safePageHtml(reply.html) : ''), [reply])
  const reducedMotion = useMemo(
    () => typeof window.matchMedia === 'function' && window.matchMedia('(prefers-reduced-motion: reduce)').matches,
    [],
  )
  if (!reply) return null
  const takeQuote = () => {
    // An inert frame with an opaque origin keeps its selection to itself; fall back to the page's
    // own selection (the caption and chip), so quoting always works from this surface.
    let selected = ''
    try {
      selected = frame.current?.contentWindow?.getSelection()?.toString() ?? ''
    } catch {
      selected = ''
    }
    if (!selected) selected = window.getSelection()?.toString() ?? ''
    const quoted = quote_text(selected, QUOTE_LIMIT)
    if (quoted) {
      setQuote(quoted)
      onQuote?.(quoted)
    }
  }
  return (
    <div style={{ order: -1, display: 'flex', flexDirection: 'column', gap: 4, maxWidth: '100%' }} onMouseUp={takeQuote} onKeyUp={takeQuote}>
      <div className="page-preview-stage" style={{ height }}>
        <iframe ref={frame} className="page-preview-frame" sandbox="" srcDoc={srcDoc} title={title} loading="lazy" tabIndex={-1} />
      </div>
      {reply.truncated ? <Text size="caption" tone="secondary">{tr('This page was shortened to fit the reply.')}</Text> : null}
      {quote ? (
        <button
          type="button"
          className="page-preview-open press"
          title={tr('Copy quote')}
          style={{ alignSelf: 'flex-start', transition: reducedMotion ? undefined : 'opacity 160ms ease' }}
          onClick={() => { void window.codync.app.copy(quote) }}
        >
          ❝ {quote}
        </button>
      ) : null}
    </div>
  )
}

/**
 * A reply that carries a visual page gets an inert preview: the iframe has an empty `sandbox` (no
 * `allow-scripts`), and the HTML is sanitized so it cannot fetch anything either. The Open action
 * hands the page to the app's real artifact viewer; the preview itself does nothing.
 */
export function PagePreviewSheet({ html, visualHtml = html, title, open, close, published = false, onQuote }: {
  html: string | null
  /** The card's page, kept on screen even while the tap-to-open sheet is closed. */
  visualHtml?: string | null
  title: string
  open: () => void
  close: () => void
  /** Set when this artifact is the visual reply published above the agent's words. */
  published?: boolean
  onQuote?: (text: string) => void
}) {
  const tr = useTr()
  const srcDoc = useMemo(() => (html ? safePageHtml(html) : ''), [html])
  return (
    <>
    {published ? <VisualReply html={visualHtml} title={title} onQuote={onQuote} /> : null}
    <Sheet open={html !== null} onClose={close} width={880} maxHeight="82vh" dismissOnDim>
      {html !== null ? (
        <SheetPage>
          <ModalHeader
            title={title}
            subtitle={tr('A preview without scripts or outside content')}
            trailing={<Button onClick={open}>{tr('Open')}</Button>}
          />
          <div className="page-preview-stage">
            <iframe className="page-preview-frame" sandbox="" srcDoc={srcDoc} title={title} loading="lazy" tabIndex={-1} aria-hidden />
          </div>
        </SheetPage>
      ) : null}
    </Sheet>
    </>
  )
}

/** A written local path being previewed inside the chat instead of in the computer's own app. */
export interface LocalPreview {
  path: string
  href: string
  kind: PreviewKind
  /** `report.md:26` names a place to read; the sandboxed frame cannot scroll a foreign document. */
  line?: number
  column?: number
}

/**
 * Chat links ask this to show a local document in-app. `MarkdownText` supplies the setter; without
 * one (a Markdown view outside a chat), the link keeps its old behaviour and opens the OS app.
 */
export const LocalPreviewContext = createContext<(preview: LocalPreview) => void>(
  (preview) => window.codync.app.openExternal(preview.href),
)

/** A local PDF or HTML from a chat reply, in the same inert surface as a visual page preview. */
export function LocalFilePreviewSheet({ preview, close }: { preview: LocalPreview | null; close: () => void }) {
  const tr = useTr()
  const title = preview ? fileReferenceName(preview.path) : ''
  // A file:// frame is foreign to the renderer, so it cannot be scrolled to a line from here. The
  // hash is the narrow additive hand-off: a viewer that understands it can jump, and the header
  // always says what the agent named (OpenBot 0.78.0).
  const position = preview?.line ? `${preview.line}${preview.column ? `:${preview.column}` : ''}` : ''
  // OpenBot 0.70.0: the fragment belongs to this preview document. Joining it through the pure
  // decision keeps an in-page link inside the frame and replaces any fragment the href already had.
  const src = preview
    ? (position ? hash_target(`#line=${preview.line}`, preview.href) ?? preview.href : preview.href)
    : ''
  return (
    <Sheet open={preview !== null} onClose={close} width={880} maxHeight="82vh" dismissOnDim>
      {preview ? (
        <SheetPage>
          <ModalHeader
            title={title}
            subtitle={position
              ? `${tr('A preview without scripts or outside content')} · ${tr('Line {line}').replaceAll('{line}', () => position)}`
              : tr('A preview without scripts or outside content')}
            trailing={<Button onClick={() => window.codync.app.openExternal(src)}>{tr('Open in system app')}</Button>}
          />
          <div className="page-preview-stage">
            {/* No `allow-scripts`: a local page cannot run scripts or reach the app's own origin. */}
            <iframe className="page-preview-frame" src={src} sandbox="" title={title} loading="lazy" tabIndex={-1} aria-hidden />
          </div>
          <Text size="caption" tone="secondary">{tr('If the document does not appear here, open it in its own app.')}</Text>
        </SheetPage>
      ) : null}
    </Sheet>
  )
}
