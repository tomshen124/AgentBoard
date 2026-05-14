// TaskLoop Policy Enforcer — bridges the Rust policy engine into the agent execution loop.
// Called before Bash/Write tool execution to enforce TaskLoop's policy decisions.
//
// FAIL-CLOSED: when TaskLoop sidecar is unavailable, high-risk tools (Bash, Write)
// require explicit user approval with a clear warning. Low-risk tools continue normally.

import { getTaskLoopClient, type ExecPlan, type FileWritePlan } from './taskloop-sidecar-client'

export interface PolicyCheck {
  allowed: boolean
  requiresApproval: boolean
  hardBlocked: boolean // true = TaskLoop explicitly denied, do NOT execute even with approval
  riskLevel: 'Low' | 'High'
  reason: string
}

// ── Exec (Bash / shell commands) ──

export async function checkExecPolicy(command: string, workingDir?: string): Promise<PolicyCheck> {
  try {
    const client = getTaskLoopClient()
    if (!client.isRunning) {
      return {
        allowed: false,
        requiresApproval: true,
        hardBlocked: false,
        riskLevel: 'High',
        reason: 'TaskLoop policy engine is not running — Bash execution requires approval'
      }
    }
    const preview = workingDir ? `cd ${workingDir} && ${command}` : command
    const plan: ExecPlan = await client.evaluateExec(preview)
    return {
      allowed: plan.allowed,
      requiresApproval: plan.requires_approval,
      hardBlocked: plan.decision === 'Deny',
      riskLevel: plan.risk_level === 'High' ? 'High' : 'Low',
      reason: plan.reason
    }
  } catch {
    // Sidecar error — fail closed for Bash
    return {
      allowed: false,
      requiresApproval: true,
      hardBlocked: false,
      riskLevel: 'High',
      reason: 'TaskLoop policy engine error — Bash execution requires approval'
    }
  }
}

// ── File Write ──

export async function checkFileWritePolicy(
  filePath: string,
  destructive: boolean = false
): Promise<PolicyCheck> {
  try {
    const client = getTaskLoopClient()
    if (!client.isRunning) {
      return {
        allowed: false,
        requiresApproval: true,
        hardBlocked: false,
        riskLevel: 'High',
        reason: 'TaskLoop policy engine is not running — file write requires approval'
      }
    }
    const plan: FileWritePlan = await client.evaluateFileWrite(filePath, destructive)
    return {
      allowed: plan.allowed,
      requiresApproval: plan.requires_approval,
      hardBlocked: plan.decision === 'Deny',
      riskLevel: plan.risk_level === 'High' ? 'High' : 'Low',
      reason: plan.reason
    }
  } catch {
    return {
      allowed: false,
      requiresApproval: true,
      hardBlocked: false,
      riskLevel: 'High',
      reason: 'TaskLoop policy engine error — file write requires approval'
    }
  }
}

// ── Record memory after tool execution ──

export async function recordToolMemory(
  kind: string,
  content: string,
  tags: string[] = []
): Promise<void> {
  try {
    const client = getTaskLoopClient()
    if (!client.isRunning) return
    await client.remember(kind, 'workspace', content, tags)
    // Persist after each memory write
    await client.saveState().catch(() => {})
  } catch {
    // Best effort — memory is not critical path
  }
}

// ── Initialize sidecar ──

let initPromise: Promise<boolean> | null = null

export async function ensureTaskLoopRunning(workspaceRoot?: string): Promise<boolean> {
  if (initPromise) {
    try {
      return await initPromise
    } catch {
      initPromise = null
    }
  }

  initPromise = (async (): Promise<boolean> => {
    const client = getTaskLoopClient()
    await client.start()
    await client.init(workspaceRoot)
    // Load persisted state if available
    await client.loadState().catch(() => {})
    return true
  })()

  try {
    return await initPromise
  } catch (err) {
    console.error('[TaskLoop] Failed to start sidecar:', err)
    initPromise = null
    return false
  }
}

export function isTaskLoopRunning(): boolean {
  return getTaskLoopClient().isRunning
}
