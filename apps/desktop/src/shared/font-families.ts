/** A picker menu with more than this many families is slower to open than it is useful. */
const MAX_FONTS = 400
const FAMILY = /^[\p{L}\p{N}][\p{L}\p{N} _+'.-]{0,79}$/u

/**
 * The family names in one platform's font-listing output. Electron has no font-list API, so each OS's
 * own command prints them: `SPFontsDataType` JSON on macOS, `fc-list` on Linux, .NET on Windows.
 */
export function fontFamilies(output: string, platform: NodeJS.Platform): string[] {
  const names: string[] = []
  if (platform === 'darwin') {
    try {
      const parsed: unknown = JSON.parse(output)
      const groups = (parsed as { SPFontsDataType?: unknown }).SPFontsDataType
      for (const entry of Array.isArray(groups) ? groups : []) {
        const name = (entry as { _name?: unknown })?._name
        if (typeof name === 'string') names.push(name)
      }
    } catch {
      return []
    }
  } else {
    for (const line of output.split(/\r?\n/)) {
      for (const name of line.split(',')) names.push(name.trim().replace(/^['"]|['"]$/g, ''))
    }
  }
  return [...new Set(names.filter((name) => FAMILY.test(name)))]
    .sort((a, b) => a.localeCompare(b))
    .slice(0, MAX_FONTS)
}
