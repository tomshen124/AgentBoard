import * as React from 'react'
import { Bot, Code2, FileText, MoreHorizontal, Search, Sparkles, Terminal, Zap } from 'lucide-react'
import { Button } from '@renderer/components/ui/button'
import { useUIStore } from '@renderer/stores/ui-store'

const AGENTS = [
  {
    name: 'Code Reviewer',
    icon: Code2,
    description: 'Expert code reviewer that enforces quality, security, and best practices.',
    tags: ['Engineering', 'Quality']
  },
  {
    name: 'Project Planner',
    icon: Sparkles,
    description: 'Breaks down goals into clear plans, tasks, and milestones.',
    tags: ['Planning', 'Strategy']
  },
  {
    name: 'Document Drafter',
    icon: FileText,
    description: 'Creates structured documents, reports, and release notes.',
    tags: ['Writing', 'Office']
  },
  {
    name: 'Research Analyst',
    icon: Search,
    description: 'Finds, evaluates, and synthesizes information from multiple sources.',
    tags: ['Research', 'Analysis']
  }
] as const

const LINKED_SKILLS = [
  ['Static Analysis', 'Run linters and static checks'],
  ['Security Scan', 'Check common vulnerabilities'],
  ['Best Practices', 'Apply language best practices'],
  ['Diff Explorer', 'Analyze and summarize changes']
] as const

export function StudioPage(): React.JSX.Element {
  const selectedAgent = AGENTS[0]
  const SelectedIcon = selectedAgent.icon

  return (
    <div className="h-full min-w-0 overflow-auto bg-background px-7 py-6">
      <div className="mx-auto flex w-full max-w-[1180px] flex-col gap-5">
        <header className="border-b border-border/60 pb-4">
          <h1 className="text-[28px] font-semibold tracking-tight text-foreground">Studio</h1>
          <p className="mt-1 text-sm text-muted-foreground">Build reusable agent behavior.</p>
          <div className="mt-5 flex gap-8 text-sm">
            {['Agents', 'Commands', 'Skills', 'Canvas'].map((item, index) => (
              <button
                key={item}
                type="button"
                className={
                  index === 0
                    ? 'border-b-2 border-foreground pb-2 font-medium text-foreground'
                    : 'pb-2 text-muted-foreground hover:text-foreground'
                }
                onClick={() => {
                  if (item === 'Skills') useUIStore.getState().openSkillsPage()
                  if (item === 'Canvas') useUIStore.getState().openDrawPage()
                  if (item === 'Commands') useUIStore.getState().openResourcesPage()
                }}
              >
                {item}
              </button>
            ))}
          </div>
        </header>

        <div className="flex items-center justify-between gap-4">
          <p className="text-sm text-muted-foreground">
            Agents combine instructions, context, and skills to deliver consistent results.
          </p>
          <Button size="sm" className="h-9 rounded-md">
            <Bot className="mr-2 size-4" />
            New Agent
          </Button>
        </div>

        <section className="grid gap-4 lg:grid-cols-4">
          {AGENTS.map((agent, index) => {
            const Icon = agent.icon
            return (
              <button
                key={agent.name}
                type="button"
                className={`rounded-xl border bg-background p-4 text-left shadow-sm transition-colors hover:bg-muted/30 ${
                  index === 0 ? 'border-primary/60' : 'border-border/70'
                }`}
              >
                <div className="mb-4 flex items-start justify-between">
                  <div className="flex size-10 items-center justify-center rounded-lg border border-border/70 bg-muted/35 text-primary">
                    <Icon className="size-5" />
                  </div>
                  <MoreHorizontal className="size-4 text-muted-foreground" />
                </div>
                <h2 className="font-semibold text-foreground">{agent.name}</h2>
                <p className="mt-3 text-xs uppercase tracking-wide text-muted-foreground">
                  Role + constraints
                </p>
                <p className="mt-2 min-h-[42px] text-sm leading-6 text-muted-foreground">
                  {agent.description}
                </p>
                <div className="mt-4 flex flex-wrap gap-2">
                  {agent.tags.map((tag) => (
                    <span
                      key={tag}
                      className="rounded bg-muted px-2 py-1 text-xs text-foreground/75"
                    >
                      {tag}
                    </span>
                  ))}
                </div>
              </button>
            )
          })}
        </section>

        <section className="rounded-xl border border-border/70 bg-background shadow-sm">
          <div className="flex items-center justify-between border-b border-border/60 p-4">
            <div className="flex items-center gap-3">
              <div className="flex size-12 items-center justify-center rounded-lg border border-primary/30 bg-primary/10 text-primary">
                <SelectedIcon className="size-6" />
              </div>
              <div>
                <div className="flex items-center gap-2">
                  <h2 className="text-xl font-semibold text-foreground">{selectedAgent.name}</h2>
                  <span className="rounded bg-emerald-500/10 px-2 py-1 text-xs font-medium text-emerald-600">
                    Active
                  </span>
                </div>
                <p className="mt-1 text-sm text-muted-foreground">{selectedAgent.description}</p>
              </div>
            </div>
            <Button variant="outline" size="sm" className="h-9 rounded-md">
              Edit agent
            </Button>
          </div>

          <div className="grid gap-4 p-4 xl:grid-cols-[minmax(0,1fr)_360px]">
            <div className="space-y-4">
              <div className="rounded-lg border border-border/70 p-4">
                <div className="mb-3 flex items-center justify-between">
                  <h3 className="text-sm font-medium">Instructions (Markdown)</h3>
                  <Button variant="ghost" size="sm" className="h-7 text-xs">
                    Copy
                  </Button>
                </div>
                <pre className="max-h-[220px] overflow-auto rounded-md bg-muted/35 p-4 text-xs leading-5 text-muted-foreground">
                  {`You are a senior code reviewer.

Your goals:
- Improve code quality, readability, and maintainability.
- Identify bugs, edge cases, and performance issues.
- Enforce secure coding practices and standards.
- Provide clear, actionable feedback.`}
                </pre>
              </div>

              <div className="rounded-lg border border-border/70 p-4">
                <h3 className="mb-3 text-sm font-medium">Test this agent</h3>
                <div className="flex min-h-[64px] items-center rounded-lg border border-border/70 px-3 text-sm text-muted-foreground">
                  Ask Code Reviewer anything...
                  <Button className="ml-auto size-9 rounded-md" size="icon">
                    <Zap className="size-4" />
                  </Button>
                </div>
              </div>
            </div>

            <aside className="space-y-4">
              <div className="rounded-lg border border-border/70 p-4">
                <div className="mb-3 flex items-center justify-between">
                  <h3 className="text-sm font-medium">Variables</h3>
                  <Button variant="ghost" size="sm" className="h-7 text-xs">
                    Edit
                  </Button>
                </div>
                <div className="space-y-3 text-sm">
                  {['language', 'framework', 'audience', 'strictness'].map((item) => (
                    <div key={item} className="flex gap-3">
                      <span className="w-24 rounded bg-muted px-2 py-1 font-mono text-xs">
                        {item}
                      </span>
                      <span className="text-muted-foreground">Configurable agent input</span>
                    </div>
                  ))}
                </div>
              </div>

              <div className="rounded-lg border border-border/70 p-4">
                <div className="mb-3 flex items-center justify-between">
                  <h3 className="text-sm font-medium">Linked Skills</h3>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-7 text-xs"
                    onClick={() => useUIStore.getState().openSkillsPage()}
                  >
                    Manage
                  </Button>
                </div>
                <div className="space-y-3">
                  {LINKED_SKILLS.map(([name, desc]) => (
                    <div key={name} className="flex gap-3 text-sm">
                      <Terminal className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
                      <div>
                        <div className="font-medium text-foreground">{name}</div>
                        <div className="text-xs text-muted-foreground">{desc}</div>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </aside>
          </div>
        </section>
      </div>
    </div>
  )
}
