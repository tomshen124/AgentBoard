import type { BuiltinProviderPreset } from './types'

export const ollamaPreset: BuiltinProviderPreset = {
  builtinId: 'ollama',
  name: 'Ollama',
  type: 'openai-chat',
  defaultBaseUrl: 'http://localhost:11434/v1',
  homepage: 'https://ollama.com',
  apiKeyUrl: 'https://ollama.com/download',
  docsUrl: 'https://github.com/ollama/ollama/blob/main/docs/api.md',
  defaultModels: [],
  requiresApiKey: false
}
