import { createContext, useMemo } from 'react'
import { Button } from '../../components/Controls'
import { ModalHeader, Sheet, SheetPage } from '../../components/Overlay'
import { Text } from '../../components/Text'
import { fileReferenceName, type PreviewKind } from '../../lib/chat-file-link.ts'
import { useTr } from '../../lib/use-language'
import { safePageHtml } from '../../lib/page-preview'
import './page-preview.css'

/**
 * A reply that carries a visual page gets an inert preview: the iframe has an empty `sandbox` (no
 * `allow-scripts`), and the HTML is sanitized so it cannot fetch anything either. The Open action
 * hands the page to the app's real artifact viewer; the preview itself does nothing.
 */
export function PagePreviewSheet({ html, title, open, close }: {
  html: string | null
  title: string
  open: () => void
  close: () => void
}) {
  const tr = useTr()
  const srcDoc = useMemo(() => (html ? safePageHtml(html) : ''), [html])
  return (
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
  const src = preview ? `${preview.href}${position ? `#line=${preview.line}` : ''}` : ''
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
