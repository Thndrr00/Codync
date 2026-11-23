// File paths an agent writes in a chat reply (OpenBot nightly triage-12380). A full path to a
// document of any type opens as a file link; a bare name or a network path still needs a known
// file type, and a command line or a flag stays plain code. Pure: no renderer or bridge here.

const ABSOLUTE = /^(?:~[/\\]|[/\\]|[A-Za-z]:[/\\])/
const UNC = /^[/\\]{2}/
const CONTROL = /[\0\r\n]/
const EXTENSION = /^[^.].*\.[\p{L}\p{N}_-]*\p{L}[\p{L}\p{N}_-]*$/u
const FLAG = /\s-+\p{L}/u
const BIN = /[/\\]s?bin[/\\]/iu

/** The known types a file can be named by without a full path (kept for network shares). */
const KNOWN = /\.(?:avif|bash|c|conf|cpp|cs|css|csv|doc|docx|env|fish|gif|go|h|heic|hpp|html?|ini|java|jpe?g|js|json|jsx|kt|kts|log|markdown|md|odt|pdf|php|png|ps1|py|rb|rs|rtf|scala|sh|sql|svg|swift|toml|ts|tsx|txt|webp|xml|ya?ml|zsh)$/iu

/** The last component of a written path, with any percent escapes decoded. */
export function fileReferenceName(path: string): string {
  const name = path.replaceAll('\\', '/').split('/').at(-1) ?? ''
  try {
    return decodeURIComponent(name)
  } catch {
    return name
  }
}

/** A name the chat already knows is a file, even without a full path. */
export function isKnownFileReference(value: string): boolean {
  if (!value || CONTROL.test(value) || /[/\\]$/.test(value)) return false
  const name = fileReferenceName(value)
  if (/^(?:Dockerfile|Makefile|\.gitignore)$/iu.test(name)) return true
  return KNOWN.test(name)
}

/**
 * A full path whose last component is a document of any type. A path with spaces is a file unless
 * it carries a command-line flag (" -v") or starts with a command in bin/sbin.
 */
export function isAbsoluteDocumentPath(value: string): boolean {
  if (!ABSOLUTE.test(value) || UNC.test(value) || CONTROL.test(value)) return false
  if (!EXTENSION.test(fileReferenceName(value))) return false
  if (!/\s/u.test(value)) return true
  const first = value.split(/\s/u)[0] ?? ''
  return !FLAG.test(value) && !BIN.test(first)
}

/** A chat file path and the URL used to open it, or null for text that stays as written. */
export function chatFileLink(value: string): { path: string; href: string } | null {
  const path = value.trim()
  if (!path) return null
  if (UNC.test(path)) return isKnownFileReference(path) ? { path, href: chatFileHref(path) } : null
  // A bare name has no workspace to resolve against here, so it stays plain code.
  if (!ABSOLUTE.test(path) || !isAbsoluteDocumentPath(path)) return null
  return { path, href: chatFileHref(path) }
}

/** The target of `[label](target)`; CommonMark needs `<…>` when the target holds spaces. */
export function linkTarget(raw: string): string {
  const target = raw.trim()
  if (target.length > 1 && target.startsWith('<') && target.endsWith('>')) return target.slice(1, -1)
  return target.split(/\s/u)[0] ?? target
}

/** A local path as a file URL: Windows separators become slashes and reserved URL chars are escaped. */
export function chatFileHref(path: string): string {
  const normalized = path.replaceAll('\\', '/')
  const absolute = /^[A-Za-z]:/.test(normalized) ? `/${normalized}` : normalized
  const escaped = encodeURI(absolute).replaceAll('#', '%23').replaceAll('?', '%3F')
  return `file://${escaped}`
}
