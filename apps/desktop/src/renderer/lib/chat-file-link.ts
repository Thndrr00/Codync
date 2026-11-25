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
export function chatFileLink(value: string, candidates: readonly string[] = []): { path: string; href: string } | null {
  const path = value.trim()
  if (!path) return null
  if (UNC.test(path)) return isKnownFileReference(path) ? { path, href: chatFileHref(path) } : null
  if (ABSOLUTE.test(path)) return isAbsoluteDocumentPath(path) ? { path, href: chatFileHref(path) } : null
  // A written path with folders must agree with a file the chat already knows; a bare name may
  // still match by name alone (OpenBot 0.78.0). Pure, so the note is decided without a DOM.
  return match_file_link(path, candidates)
}

/** The folders and name of a written path, with separators unified and percent escapes decoded. */
function pathParts(path: string): string[] {
  const normalized = path.trim().replaceAll('\\', '/').replace(/^[A-Za-z]:/, '').replace(/^\/+/, '')
  return normalized.split('/').filter(Boolean).map((part) => {
    try {
      return decodeURIComponent(part)
    } catch {
      return part
    }
  })
}

/**
 * Resolve a written file path against the paths the chat already knows (OpenBot 0.78.0). A path
 * that carries folders only matches a candidate whose every folder agrees, so
 * `other-repo/skills/x/SKILL.md` cannot open a previously seen `SKILL.md`. A bare name still matches
 * by name. No match returns null and the link handler shows a "File not found" note. Pure, tested.
 */
export function match_file_link(requested: string, candidates: readonly string[]): { path: string; href: string } | null {
  if (!requested || CONTROL.test(requested)) return null
  const wanted = pathParts(requested)
  const name = wanted.at(-1)
  if (!name) return null
  const folders = wanted.slice(0, -1).join('/').toLowerCase()
  const matches = candidates.flatMap((candidate) => {
    const parts = pathParts(candidate)
    if (parts.at(-1) !== name) return []
    // A bare name agrees with any candidate's folders. A written chain must agree with the whole
    // chain or its end — a path relative to the project still names the same file — never just the
    // file name when the folders differ.
    const knownFolders = parts.slice(0, -1).join('/').toLowerCase()
    if (folders && knownFolders !== folders && !knownFolders.endsWith(`/${folders}`)) return []
    return [{ candidate, depth: parts.length - 1 }]
  })
  // When several files share a name, the least nested one is the least surprising bare-name match.
  const match = matches.reduce<(typeof matches)[number] | null>(
    (best, item) => (!best || item.depth < best.depth ? item : best), null,
  )
  return match ? { path: match.candidate, href: chatFileHref(match.candidate) } : null
}

/** Whether a written target is a local relative file path whose link should report a missing file. */
export function isMissingFileCandidate(value: string): boolean {
  if (!value || CONTROL.test(value) || ABSOLUTE.test(value) || UNC.test(value)) return false
  if (/^[A-Za-z][A-Za-z0-9+.-]*:/u.test(value)) return false
  const parts = pathParts(value)
  return parts.length > 1 && KNOWN.test(parts.at(-1) ?? '')
}

/** A path with an optional `:line` or `:line:col` suffix, as agents write `report.md:26`. */
export interface FileLinkTarget {
  path: string
  line?: number
  column?: number
}

const MAX_LINE = 1_000_000

/**
 * Parse a written file target into the path to open and the line/column to show (OpenBot 0.78.0).
 * The last one or two colon-separated numbers are the position, so `C:/a/report.md:26:5` and
 * `/a/report.md:26` keep their drive and folders. A missing, non-numeric or absurd position leaves
 * just the path; there is no line to scroll to. Pure, tested.
 */
export function file_link_target(text: string): FileLinkTarget | null {
  const value = text.trim()
  if (!value || CONTROL.test(value)) return null
  const parts = value.split(':')
  const numbers: number[] = []
  while (parts.length > 1) {
    const last = parts.at(-1)!
    // A Windows drive letter or a path segment is not a line number. A non-numeric tail on a file
    // name, though, is a malformed position (`report.md:oops`), not part of the path.
    if (!/^\d+$/u.test(last)) {
      if (!numbers.length && !last.includes('/') && !last.includes('\\') && parts.length > 1 && /\./u.test(parts.at(-2)!)) return null
      break
    }
    const number = Number(last)
    if (!Number.isSafeInteger(number) || number < 1 || number > MAX_LINE) return null
    numbers.unshift(number)
    parts.pop()
    if (numbers.length === 2) break
  }
  const path = parts.join(':')
  if (!path) return null
  return numbers.length === 2 ? { path, line: numbers[0], column: numbers[1] } : numbers.length === 1 ? { path, line: numbers[0] } : { path }
}

/** The target of `[label](target)`; CommonMark needs `<…>` when the target holds spaces. */
export function linkTarget(raw: string): string {
  const target = raw.trim()
  if (target.length > 1 && target.startsWith('<') && target.endsWith('>')) return target.slice(1, -1)
  return target.split(/\s/u)[0] ?? target
}

export type PreviewKind = 'html' | 'pdf'

/**
 * Whether a written file path can be shown in the chat's sandboxed preview. HTML and PDF have an
 * in-app surface; every other type falls back to the computer's own app (OpenBot 0.77.0/0.74.0).
 */
export function previewKind(path: string): PreviewKind | null {
  if (CONTROL.test(path)) return null
  // A web URL is not a local document; a Windows drive letter is.
  if (/^[A-Za-z][A-Za-z0-9+.-]*:/u.test(path) && !/^[A-Za-z]:[/\\]/u.test(path)) return null
  const extension = fileReferenceName(path).split('.').at(-1)?.toLowerCase() ?? ''
  if (extension === 'pdf') return 'pdf'
  if (extension === 'html' || extension === 'htm') return 'html'
  return null
}

/** A local path as a file URL: Windows separators become slashes and reserved URL chars are escaped. */
export function chatFileHref(path: string): string {
  const normalized = path.replaceAll('\\', '/')
  const absolute = /^[A-Za-z]:/.test(normalized) ? `/${normalized}` : normalized
  const escaped = encodeURI(absolute).replaceAll('#', '%23').replaceAll('?', '%3F')
  return `file://${escaped}`
}
