// SSH removed — empty stub
import { create } from 'zustand'

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type SshTab = any
export interface SshSession {
  id: string
  connectionId: string
  name: string
  host: string
  status: string
  error?: string
  username?: string
}
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type SshConnection = any

export const useSshStore = create<{
  _loaded: boolean
  connections: SshConnection[]
  groups: any[]
  sessions: Record<string, any>
  tabs: SshTab[]
  openTabs: SshTab[]
  activeTabId: string | null
  loadAll: (...args: any[]) => Promise<void>
  connect: (...args: any[]) => Promise<SshSession>
  disconnect: (...args: any[]) => Promise<void>
  setActiveTab: (id: string | null) => void
  openTerminalTab: (id: string) => string | null
  closeTab: (id: string) => void
  error: string | null
}>(() => ({
  _loaded: true,
  connections: [],
  groups: [],
  sessions: {} as any,
  tabs: [],
  openTabs: [],
  activeTabId: null,
  error: null,
  loadAll: async (..._args: any[]) => {},
  connect: async () => ({ id: '', connectionId: '', name: '', host: '', status: 'disconnected' }),
  disconnect: async () => {},
  setActiveTab: () => {},
  openTerminalTab: () => null,
  closeTab: () => {}
}))
