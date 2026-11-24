import { promises as fs } from 'node:fs'
import { ipcMain, shell } from 'electron'
import { revealableFolder } from '../shared/workspace-path.ts'

/** Opens a bot workspace in the system file manager; no file or arbitrary path can reach this channel. */
export function registerWorkspaceReveal() {
  ipcMain.handle('app:revealBotWorkspace', async (_event, cwd: unknown) => {
    const path = revealableFolder(cwd)
    if (!path) throw new Error('This is not a bot workspace folder')
    const stat = await fs.stat(path).catch(() => null)
    if (!stat?.isDirectory()) throw new Error('This bot workspace folder is missing')
    const error = await shell.openPath(path)
    if (error) throw new Error(error)
  })
}
