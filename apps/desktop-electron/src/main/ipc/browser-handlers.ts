import { ipcMain, session } from 'electron'
import { BUILTIN_BROWSER_PARTITION } from '../../shared/browser-plugin'

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

export function registerBrowserHandlers(): void {
  ipcMain.handle('browser:clear-cookies', async () => {
    try {
      const browserSession = session.fromPartition(BUILTIN_BROWSER_PARTITION)
      await browserSession.clearStorageData({ storages: ['cookies'] })
      return { success: true }
    } catch (error) {
      console.error('[Browser] Failed to clear cookies:', error)
      return { success: false, error: getErrorMessage(error) }
    }
  })
}
