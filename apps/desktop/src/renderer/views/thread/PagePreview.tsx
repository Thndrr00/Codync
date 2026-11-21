import { useMemo } from 'react'
import { Button } from '../../components/Controls'
import { ModalHeader, Sheet, SheetPage } from '../../components/Overlay'
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
