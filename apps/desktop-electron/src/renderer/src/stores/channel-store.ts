// Channels removed — empty stub. Uses 'any' to avoid maintaining full type compatibility.
import { create } from 'zustand'

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type Channel = any

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function initChannelEventListener(): void {}

const stub = create<any>(() => ({
  channels: [],
  providers: [],
  selectedChannelId: null,
  channelStatuses: {},
  activeChannelIdsByProject: {},

  loadProviders: async () => {},
  loadChannels: async () => {},

  addChannel: async () => '',
  updateChannel: async () => {},
  removeChannel: async () => {},
  toggleChannelEnabled: async () => {},

  startChannel: async () => undefined,
  stopChannel: async () => {},
  refreshChannelStatus: async () => {},

  setSelectedChannel: () => {},

  toggleActiveChannel: () => {},
  clearActiveChannels: () => {},
  getActiveChannelIds: () => [] as string[],

  channelSessions: {},
  loadChannelSessions: async () => {},

  getDescriptor: () => undefined,
  getConfiguredChannels: () => [] as any[],
  getActiveChannels: () => [] as any[]
}))

export const useChannelStore = stub
