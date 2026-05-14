// App plugins removed - compatibility stub.
import { create } from 'zustand'

import type { AppPluginId, AppPluginInstance } from '@renderer/lib/app-plugin/types'

type ImagePluginConfig = {
  provider: string
  model: string
  apiKey?: string
  baseUrl?: string
}

type AppPluginStoreStub = {
  plugins: AppPluginInstance[]
  tools: never[]
  loaded: boolean
  loadAll: () => Promise<void>
  getPlugin: (id: AppPluginId, projectId?: string | null) => AppPluginInstance | null
  getResolvedImagePluginConfig: () => ImagePluginConfig | null
  isImageToolAvailable: () => boolean
  isBrowserToolAvailable: () => boolean
  isDesktopControlToolAvailable: () => boolean
}

export const useAppPluginStore = create<any>(
  () =>
    ({
      plugins: [],
      tools: [],
      loaded: true,
      loadAll: async () => {},
      getPlugin: () => null,
      getResolvedImagePluginConfig: () => null,
      isImageToolAvailable: () => false,
      isBrowserToolAvailable: () => false,
      isDesktopControlToolAvailable: () => false
    }) satisfies AppPluginStoreStub
)

export function initAppPluginStore(): void {}
