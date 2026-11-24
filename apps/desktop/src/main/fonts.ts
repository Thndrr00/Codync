import { execFile } from 'node:child_process'
import { ipcMain } from 'electron'
import { promisify } from 'node:util'
import { fontFamilies } from '../shared/font-families.ts'

const run = promisify(execFile)

/** Queried once per launch: installed fonts don't change while the app is open. */
async function installedFonts(platform: NodeJS.Platform): Promise<string[]> {
  try {
    if (platform === 'darwin') {
      const { stdout } = await run('/usr/sbin/system_profiler', ['SPFontsDataType', '-json'], { timeout: 8000, maxBuffer: 16 * 1024 * 1024 })
      return fontFamilies(stdout, platform)
    }
    if (platform === 'linux') {
      const { stdout } = await run('fc-list', ['--format=%{family}\\n'], { timeout: 8000, maxBuffer: 16 * 1024 * 1024 })
      return fontFamilies(stdout, platform)
    }
    if (platform === 'win32') {
      const { stdout } = await run('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', 'Add-Type -AssemblyName System.Drawing; [System.Drawing.Text.InstalledFontCollection]::new().Families | ForEach-Object { $_.Name }'], { timeout: 8000, maxBuffer: 16 * 1024 * 1024 })
      return fontFamilies(stdout, platform)
    }
  } catch {
    // A missing system tool leaves the pickers with just the curated default.
  }
  return []
}

let cached: Promise<string[]> | null = null

/** Installed family names for Settings › Appearance's interface and code font pickers. */
export function registerFonts() {
  ipcMain.handle('app:installedFonts', () => {
    cached ??= installedFonts(process.platform)
    return cached
  })
}
