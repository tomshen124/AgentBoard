// TaskLoop Context Injector — enriches agent loop with structured memory and workspace context.

import { getTaskLoopClient, type PromptContextBundle } from './taskloop-sidecar-client'

const TOKEN_BUDGET_SYSTEM_PERCENT = 0.15 // Reserve 15% for system/context
const TOKEN_BUDGET_OUTPUT_RESERVE = 0.15 // Reserve 15% for model output
const DEFAULT_MODEL_TOKENS = 128_000

// ── Context assembly ──

export interface EnrichedSystemPrompt {
  systemPrompt: string
  tokenEstimate: number
  memoryCount: number
}

export async function buildEnrichedSystemPrompt(
  basePrompt: string,
  taskId?: string
): Promise<EnrichedSystemPrompt> {
  try {
    const client = getTaskLoopClient()
    if (!client.isRunning) {
      return { systemPrompt: basePrompt, tokenEstimate: 0, memoryCount: 0 }
    }

    // Fetch workspace context from TaskLoop
    const bundle: PromptContextBundle = await client.assembleContext(taskId)

    // Fetch recent workspace memories
    const memories = await client.recall('workspace')

    let enriched = basePrompt

    // Inject TaskLoop context sections
    if (bundle.sections.length > 0) {
      enriched += '\n\n<!-- TaskLoop Workspace Context -->\n'
      for (const section of bundle.sections) {
        if (section.content.trim()) {
          enriched += `\n## ${section.title}\n${section.content}\n`
        }
      }
    }

    // Inject structured memory
    if (memories.length > 0) {
      enriched += '\n<!-- TaskLoop Memory -->\n'
      enriched += '## Previous Context (from workspace memory)\n'
      for (const mem of memories.slice(0, 10)) {
        const tagStr = mem.tags.length > 0 ? ` [${mem.tags.join(', ')}]` : ''
        enriched += `- (${mem.kind}${tagStr}) ${mem.content}\n`
      }
    }

    // Estimate tokens (rough: ~4 chars per token for CJK-friendly estimate)
    const addedChars = enriched.length - basePrompt.length
    const tokenEstimate = Math.ceil(addedChars / 3)

    return {
      systemPrompt: enriched,
      tokenEstimate,
      memoryCount: memories.length
    }
  } catch {
    return { systemPrompt: basePrompt, tokenEstimate: 0, memoryCount: 0 }
  }
}

// ── Memory recording ──

export async function recordTurnMemory(
  role: 'assistant' | 'tool',
  summary: string,
  tags: string[] = []
): Promise<void> {
  try {
    const client = getTaskLoopClient()
    if (!client.isRunning) return
    const kind = role === 'tool' ? 'fact' : 'decision'
    await client.remember(kind, 'workspace', summary, tags)
  } catch {
    // best effort
  }
}

// ── Token budget tracking ──

interface BudgetState {
  systemTokens: number
  conversationTokens: number
  modelLimit: number
}

let currentBudget: BudgetState = {
  systemTokens: 0,
  conversationTokens: 0,
  modelLimit: DEFAULT_MODEL_TOKENS
}

export function initTokenBudget(
  systemEstimate: number,
  modelContextLimit: number = DEFAULT_MODEL_TOKENS
): void {
  currentBudget = {
    systemTokens: systemEstimate,
    conversationTokens: 0,
    modelLimit: modelContextLimit
  }
}

export function trackConversationTokens(tokens: number): void {
  currentBudget.conversationTokens += tokens
}

export function shouldCompact(): boolean {
  const reserve = Math.floor(
    currentBudget.modelLimit * (TOKEN_BUDGET_SYSTEM_PERCENT + TOKEN_BUDGET_OUTPUT_RESERVE)
  )
  const available = currentBudget.modelLimit - currentBudget.systemTokens - reserve
  return currentBudget.conversationTokens > available
}

export function getBudgetInfo(): {
  used: number
  limit: number
  percent: number
  shouldCompact: boolean
} {
  const total = currentBudget.systemTokens + currentBudget.conversationTokens
  return {
    used: total,
    limit: currentBudget.modelLimit,
    percent: Math.round((total / currentBudget.modelLimit) * 100),
    shouldCompact: shouldCompact()
  }
}

// ── Simple token estimator (chars / 3 for mixed content) ──

export function estimateTokens(text: string): number {
  return Math.max(1, Math.ceil(text.length / 3))
}
