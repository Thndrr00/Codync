import { fileReferenceName } from '../../lib/chat-file-link.ts'

/** What a chat directory entry knows about a PDF: its display name, path and internal id. */
export interface PdfSaveEntry {
  name?: string | null
  path?: string | null
  id?: string | null
}

const RANDOM_ID = /^[0-9a-f-]{8,}$/iu
const HAS_LETTER = /\p{L}/u

/**
 * The name offered in a PDF save dialog (triage-17392). A chat entry's display name wins over the
 * internal id the host uses as a key, so a PDF opened from a chat link is saved under the name the
 * agent wrote, not `a3f9c1…`. Pure, so the rule can be tested without a dialog or a DOM.
 */
export function pdf_save_name(entry: PdfSaveEntry | null, fallback: string): string {
  const name = entry?.name?.trim()
  if (name && HAS_LETTER.test(name) && !RANDOM_ID.test(name)) return fileReferenceName(name)
  const path = entry?.path?.trim()
  if (path && HAS_LETTER.test(path)) return fileReferenceName(path)
  return fileReferenceName(fallback) || 'document.pdf'
}
