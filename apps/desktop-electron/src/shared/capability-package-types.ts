export type CapabilityPackageSource = 'builtin' | 'marketplace' | 'local'

export interface CapabilityPackageSkillRef {
  name: string
  description?: string
  path?: string
  enabledByDefault?: boolean
}

export interface CapabilityPackageHookRef {
  event:
    | 'before_agent_run'
    | 'after_agent_run'
    | 'before_tool_call'
    | 'after_tool_call'
    | 'on_approval_request'
  command: string
  description?: string
  enabledByDefault?: boolean
}

export interface CapabilityPackageMcpServerRef {
  name: string
  transport: 'stdio' | 'http' | 'sse'
  command?: string
  args?: string[]
  url?: string
  enabledByDefault?: boolean
}

export interface CapabilityPackageManifest {
  id: string
  name: string
  version: string
  source: CapabilityPackageSource
  description?: string
  skills?: CapabilityPackageSkillRef[]
  hooks?: CapabilityPackageHookRef[]
  mcpServers?: CapabilityPackageMcpServerRef[]
}

export function isBuiltinCapabilityPackageId(packageId: string): boolean {
  return packageId.endsWith('@builtin')
}
