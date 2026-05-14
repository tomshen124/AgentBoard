import type { IPCClient } from '../tools/tool-types'

/**
 * IPC Client wrapper for renderer process.
 * Wraps Electron's ipcRenderer with typed interface.
 */
class ElectronIPCClient implements IPCClient {
  async invoke(channel: string, ...args: unknown[]): Promise<unknown> {
    return window.electron.ipcRenderer.invoke(channel, ...args)
  }

  send(channel: string, ...args: unknown[]): void {
    window.electron.ipcRenderer.send(channel, ...args)
  }

  on(channel: string, callback: (...args: unknown[]) => void): () => void {
    const handler = (_event: unknown, ...args: unknown[]): void => {
      callback(...args)
    }
    window.electron.ipcRenderer.on(channel, handler)
    return () => {
      window.electron.ipcRenderer.removeListener(channel, handler)
    }
  }
}

export const ipcClient: IPCClient = new ElectronIPCClient()
