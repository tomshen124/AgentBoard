// TaskLoop Sidecar Client — spawns the Rust sidecar binary and communicates via JSON-RPC over stdin/stdout.

import { app } from 'electron'
import { spawn, type ChildProcess } from 'child_process'
import * as os from 'os'
import * as path from 'path'

// ── RPC Wire Types ──

interface RpcRequest {
  jsonrpc: '2.0'
  id: string | number
  method: string
  params: unknown
}

interface RpcResponse {
  jsonrpc: '2.0'
  id: string | number
  result?: unknown
  error?: { code: number; message: string }
}

// ── Response Types ──

export interface WorkspaceSnapshot {
  root: string
  contracts_loaded: boolean
  policy: {
    writable_roots: string[]
    backup_before_write: boolean
    destructive_requires_approval: boolean
    exec_requires_approval: boolean
    allowed_exec_languages: string[]
  }
}

export interface ExecPlan {
  decision: string
  allowed: boolean
  requires_approval: boolean
  risk_level: string
  interpreter?: string
  reason: string
}

export interface FileWritePlan {
  decision: string
  allowed: boolean
  requires_approval: boolean
  risk_level: string
  normalized_path: string
  reason: string
}

export interface MemoryRecord {
  id: string
  kind: string
  scope: string
  content: string
  source: string
  confidence: number
  updated_at_ms: number
  tags: string[]
}

export interface PromptContextSection {
  key: string
  title: string
  source: string
  content: string
}

export interface PromptContextBundle {
  generated_at_ms: number
  task_id: string | null
  sections: PromptContextSection[]
}

export interface TaskInfo {
  id: string
  session_id: string
  title: string
  kind: string
  priority: string
  created_at_ms: number
}

export interface TaskProjection {
  task_id: string
  session_id: string
  title: string
  lifecycle: string
  workflow_mode: string
  health: string
  phase?: string
  current_step?: string
}

export interface DirEntry {
  name: string
  is_dir: boolean
  size_bytes: number
}

export interface SearchMatch {
  file: string
  line: number
  content: string
}

// ── Client ──

export class TaskLoopSidecarClient {
  private process: ChildProcess | null = null
  private pending = new Map<string, { resolve: (v: unknown) => void; reject: (e: Error) => void }>()
  private buffer = ''
  private nextId = 1
  private binaryPath: string

  constructor(binaryPath?: string) {
    this.binaryPath = binaryPath ?? this.defaultBinaryPath()
  }

  private defaultBinaryPath(): string {
    // In development, the binary is at ../../target/debug/taskloop-sidecar
    // In production, it should be bundled alongside the app
    if (!app.isPackaged) {
      return path.join(__dirname, '..', '..', '..', '..', 'target', 'debug', 'taskloop-sidecar')
    }
    // Production path: bundled in app resources
    return path.join(process.resourcesPath, 'bin', 'taskloop-sidecar')
  }

  async start(): Promise<void> {
    if (this.process) return

    return new Promise((resolve, reject) => {
      const child = spawn(this.binaryPath, [], {
        stdio: ['pipe', 'pipe', 'pipe'],
        env: { ...process.env }
      })

      child.stderr?.on('data', (data: Buffer) => {
        process.stderr.write(`[taskloop-sidecar] ${data}`)
      })

      child.on('error', (err) => {
        this.process = null
        reject(new Error(`Failed to start sidecar: ${err.message}`))
      })

      child.on('close', (code) => {
        this.process = null
        // Reject all pending
        for (const [, pending] of this.pending) {
          pending.reject(new Error(`Sidecar exited with code ${code}`))
        }
        this.pending.clear()
      })

      child.stdout?.on('data', (data: Buffer) => {
        this.buffer += data.toString()
        this.processBuffer()
      })

      // Give it a tick to start
      setTimeout(() => {
        this.process = child
        resolve()
      }, 100)
    })
  }

  get isRunning(): boolean {
    return this.process !== null && !this.process.killed
  }

  async stop(): Promise<void> {
    if (!this.process) return
    const child = this.process
    this.process = null
    child.kill()
  }

  // ── RPC call ──

  private async call(method: string, params: unknown = {}): Promise<unknown> {
    if (!this.process || this.process.killed) {
      await this.start()
    }

    const id = this.nextId++
    const request: RpcRequest = { jsonrpc: '2.0', id, method, params }

    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(String(id))
        reject(new Error(`RPC timeout: ${method}`))
      }, 30_000)

      this.pending.set(String(id), {
        resolve: (v) => {
          clearTimeout(timer)
          resolve(v)
        },
        reject: (e) => {
          clearTimeout(timer)
          reject(e)
        }
      })

      this.process!.stdin!.write(JSON.stringify(request) + '\n')
    })
  }

  private processBuffer(): void {
    const lines = this.buffer.split('\n')
    this.buffer = lines.pop() ?? ''

    for (const line of lines) {
      if (!line.trim()) continue
      try {
        const resp: RpcResponse = JSON.parse(line)
        const pending = this.pending.get(String(resp.id))
        if (!pending) continue

        this.pending.delete(String(resp.id))
        if (resp.error) {
          pending.reject(new Error(resp.error.message))
        } else {
          pending.resolve(resp.result)
        }
      } catch {
        // skip parse errors in buffer processing
      }
    }
  }

  // ── Public API ──

  async init(root: string = os.homedir()): Promise<WorkspaceSnapshot> {
    return this.call('workspace.init', { root }) as Promise<WorkspaceSnapshot>
  }

  async snapshot(): Promise<WorkspaceSnapshot> {
    return this.call('workspace.snapshot') as Promise<WorkspaceSnapshot>
  }

  async ensureSession(id: string): Promise<{ id: string }> {
    return this.call('session.ensure', { id }) as Promise<{ id: string }>
  }

  async createTask(sessionId: string, title: string, prompt: string): Promise<TaskInfo> {
    return this.call('task.create', {
      session_id: sessionId,
      title,
      prompt
    }) as Promise<TaskInfo>
  }

  async getTaskProjection(taskId: string): Promise<TaskProjection> {
    return this.call('task.projection', { task_id: taskId }) as Promise<TaskProjection>
  }

  async evaluateExec(command: string): Promise<ExecPlan> {
    return this.call('policy.evaluate_exec', { command }) as Promise<ExecPlan>
  }

  async evaluateFileWrite(filePath: string, destructive: boolean = false): Promise<FileWritePlan> {
    return this.call('policy.evaluate_file_write', {
      path: filePath,
      destructive
    }) as Promise<FileWritePlan>
  }

  async readFile(filePath: string, offset: number = 0, limit: number = 200): Promise<string> {
    return this.call('tools.read_file', {
      path: filePath,
      offset,
      limit
    }) as Promise<string>
  }

  async listDir(dirPath: string): Promise<DirEntry[]> {
    return this.call('tools.list_dir', { path: dirPath }) as Promise<DirEntry[]>
  }

  async searchRepo(
    query: string,
    dirPath: string = '.',
    maxResults: number = 20
  ): Promise<SearchMatch[]> {
    return this.call('tools.search_repo', {
      query,
      path: dirPath,
      max_results: maxResults
    }) as Promise<SearchMatch[]>
  }

  async remember(
    kind: string,
    scope: string,
    content: string,
    tags: string[] = []
  ): Promise<string> {
    return this.call('memory.remember', { kind, scope, content, tags }) as Promise<string>
  }

  async recall(scope?: string, kind?: string): Promise<MemoryRecord[]> {
    return this.call('memory.recall', { scope, kind }) as Promise<MemoryRecord[]>
  }

  async assembleContext(taskId?: string): Promise<PromptContextBundle> {
    return this.call('context.assemble', { task_id: taskId }) as Promise<PromptContextBundle>
  }

  async approveExec(taskId: string, checkpoint: string): Promise<string> {
    return this.call('approval.approve', { task_id: taskId, checkpoint }) as Promise<string>
  }

  async rejectExec(taskId: string, checkpoint: string, reason: string = ''): Promise<string> {
    return this.call('approval.reject', {
      task_id: taskId,
      checkpoint,
      reason
    }) as Promise<string>
  }

  async saveState(): Promise<string> {
    return this.call('state.save') as Promise<string>
  }

  async loadState(): Promise<boolean> {
    return this.call('state.load') as Promise<boolean>
  }
}

// Singleton
let clientInstance: TaskLoopSidecarClient | null = null

export function getTaskLoopClient(): TaskLoopSidecarClient {
  if (!clientInstance) {
    clientInstance = new TaskLoopSidecarClient()
  }
  return clientInstance
}
