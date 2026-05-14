export const REMOTE_TOOL_PERMISSION_PROTOCOL = 'agentboard.remote-tool.permission.v1'

export type RemoteToolPermissionSource = 'team-runtime' | 'remote-runner' | 'mcp-proxy' | 'ssh-runner'

export interface RemoteToolPermissionRequest {
  protocol: typeof REMOTE_TOOL_PERMISSION_PROTOCOL
  requestId: string
  toolCallId?: string
  toolName: string
  input: Record<string, unknown>
  source: RemoteToolPermissionSource
  runId?: string
  risk?: 'low' | 'medium' | 'high'
  summary?: string
}

export interface RemoteToolPermissionResponse {
  protocol: typeof REMOTE_TOOL_PERMISSION_PROTOCOL
  requestId: string
  approved: boolean
  reason?: string
}

export function isRemoteToolPermissionRequest(
  value: unknown
): value is RemoteToolPermissionRequest {
  if (!value || typeof value !== 'object') return false
  const candidate = value as Partial<RemoteToolPermissionRequest>
  return (
    candidate.protocol === REMOTE_TOOL_PERMISSION_PROTOCOL &&
    typeof candidate.requestId === 'string' &&
    typeof candidate.toolName === 'string' &&
    !!candidate.input &&
    typeof candidate.input === 'object' &&
    typeof candidate.source === 'string'
  )
}
