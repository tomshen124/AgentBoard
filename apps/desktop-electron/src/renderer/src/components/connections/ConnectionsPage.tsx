import * as React from 'react'
import {
  CheckCircle2,
  Cloud,
  ExternalLink,
  FolderOpen,
  Github,
  Globe2,
  Layers,
  MoreVertical,
  Play,
  RefreshCw,
  Search,
  Shield
} from 'lucide-react'
import { Button } from '@renderer/components/ui/button'
import { useUIStore } from '@renderer/stores/ui-store'

const CONNECTIONS = [
  {
    name: 'Local filesystem MCP',
    kind: 'MCP Server',
    detail: 'Filesystem',
    icon: FolderOpen,
    status: 'Connected',
    lastSeen: 'Just now'
  },
  {
    name: 'Browser tools',
    kind: 'MCP Server',
    detail: 'Browser',
    icon: Globe2,
    status: 'Connected',
    lastSeen: '5m ago'
  },
  {
    name: 'Tencent service connector',
    kind: 'Vendor Service',
    detail: 'Tencent Cloud',
    icon: Cloud,
    status: 'Connected',
    lastSeen: '1h ago'
  },
  {
    name: 'SkillHub source',
    kind: 'Skill Source',
    detail: 'HTTP',
    icon: Layers,
    status: 'Connected',
    lastSeen: '2h ago'
  },
  {
    name: 'ClawHub source',
    kind: 'Skill Source',
    detail: 'REST API',
    icon: Search,
    status: 'Connected',
    lastSeen: '1d ago'
  },
  {
    name: 'GitHub source',
    kind: 'Skill Source',
    detail: 'GitHub',
    icon: Github,
    status: 'Disconnected',
    lastSeen: '3d ago'
  }
] as const

const TOOLS = [
  ['read_file', 'Read the contents of a file from the local filesystem.'],
  ['write_file', 'Create or overwrite a file on the local filesystem.'],
  ['list_directory', 'List files and directories in a given path.'],
  ['search_files', 'Search for files by name pattern.']
] as const

const TABS = ['MCP Servers', 'Web Search', 'Browser', 'Skill Sources', 'Vendor Services'] as const

export function ConnectionsPage(): React.JSX.Element {
  const [activeTab, setActiveTab] = React.useState<(typeof TABS)[number]>('MCP Servers')
  const selected = CONNECTIONS[0]
  const SelectedIcon = selected.icon

  return (
    <div className="h-full min-w-0 overflow-auto bg-background px-7 py-6">
      <div className="mx-auto flex w-full max-w-[1180px] flex-col gap-5">
        <header className="border-b border-border/60 pb-4">
          <h1 className="text-[28px] font-semibold tracking-tight text-foreground">Connections</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            External tools, services, search, and sources.
          </p>
          <div className="mt-5 inline-flex rounded-lg border border-border/70 bg-muted/25 p-1">
            {TABS.map((tab) => (
              <button
                key={tab}
                type="button"
                className={`h-8 rounded-md px-5 text-xs transition-colors ${
                  activeTab === tab
                    ? 'bg-background text-foreground shadow-sm'
                    : 'text-muted-foreground hover:text-foreground'
                }`}
                onClick={() => setActiveTab(tab)}
              >
                {tab}
              </button>
            ))}
          </div>
        </header>

        <div className="grid gap-4 xl:grid-cols-[380px_minmax(0,1fr)]">
          <section className="rounded-xl border border-border/70 bg-background shadow-sm">
            <div className="flex items-center justify-between border-b border-border/60 p-4">
              <h2 className="text-sm font-semibold text-foreground">Connections</h2>
              <Button
                variant="outline"
                size="sm"
                className="h-8 rounded-md text-xs"
                onClick={() => useUIStore.getState().openSettingsPage('mcp')}
              >
                Add connection
              </Button>
            </div>
            <div className="divide-y divide-border/50">
              {CONNECTIONS.map((connection, index) => {
                const Icon = connection.icon
                const connected = connection.status === 'Connected'
                return (
                  <button
                    key={connection.name}
                    type="button"
                    className={`flex w-full items-center gap-3 px-4 py-4 text-left transition-colors hover:bg-muted/35 ${
                      index === 0 ? 'bg-muted/40' : ''
                    }`}
                  >
                    <Icon className="size-6 shrink-0 text-muted-foreground" />
                    <div className="min-w-0 flex-1">
                      <div className="truncate text-sm font-medium text-foreground">
                        {connection.name}
                      </div>
                      <div className="mt-1 text-xs text-muted-foreground">
                        {connection.kind} · {connection.detail}
                      </div>
                    </div>
                    <div className="text-right text-xs">
                      <div className={connected ? 'text-emerald-600' : 'text-muted-foreground'}>
                        {connected ? '● Connected' : '● Disconnected'}
                      </div>
                      <div className="mt-1 text-muted-foreground">{connection.lastSeen}</div>
                    </div>
                  </button>
                )
              })}
            </div>
          </section>

          <section className="rounded-xl border border-border/70 bg-background shadow-sm">
            <div className="flex items-center justify-between border-b border-border/60 p-4">
              <div className="flex items-center gap-3">
                <SelectedIcon className="size-8 text-foreground" />
                <div>
                  <h2 className="text-lg font-semibold text-foreground">{selected.name}</h2>
                  <p className="text-xs text-muted-foreground">
                    {selected.kind} · {selected.detail}
                  </p>
                </div>
              </div>
              <div className="flex items-center gap-2">
                <Button variant="outline" size="sm" className="h-8 rounded-md text-xs">
                  <Play className="mr-2 size-3.5" />
                  Test connection
                </Button>
                <Button variant="ghost" size="icon" className="size-8">
                  <MoreVertical className="size-4" />
                </Button>
              </div>
            </div>

            <div className="space-y-4 p-4">
              <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_280px]">
                <div className="rounded-lg border border-border/70 p-4">
                  <h3 className="mb-4 text-sm font-semibold">Status</h3>
                  <dl className="grid grid-cols-[120px_minmax(0,1fr)] gap-x-4 gap-y-3 text-sm">
                    <dt className="text-muted-foreground">Connection</dt>
                    <dd className="font-medium text-emerald-600">Connected</dd>
                    <dt className="text-muted-foreground">Server URL</dt>
                    <dd className="truncate">stdio://local-filesystem-mcp</dd>
                    <dt className="text-muted-foreground">Last tested</dt>
                    <dd>Just now</dd>
                    <dt className="text-muted-foreground">Version</dt>
                    <dd>1.0.3</dd>
                  </dl>
                </div>

                <div className="flex items-center gap-3 rounded-lg border border-emerald-500/20 bg-emerald-500/5 p-4">
                  <CheckCircle2 className="size-8 shrink-0 text-emerald-600" />
                  <div>
                    <div className="text-sm font-semibold text-foreground">Connection healthy</div>
                    <p className="mt-1 text-xs leading-5 text-muted-foreground">
                      All capabilities are available and responding.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border border-border/70 p-4">
                <div className="mb-3 flex items-center justify-between">
                  <h3 className="text-sm font-semibold">Available tools (4)</h3>
                  <Button variant="outline" size="sm" className="h-8 rounded-md text-xs">
                    <RefreshCw className="mr-2 size-3.5" />
                    Refresh
                  </Button>
                </div>
                <div className="space-y-3">
                  {TOOLS.map(([name, desc]) => (
                    <div key={name} className="flex items-center gap-3 text-sm">
                      <ZapIcon />
                      <div className="min-w-0 flex-1">
                        <div className="font-mono text-xs font-medium text-foreground">{name}</div>
                        <div className="mt-0.5 text-xs text-muted-foreground">{desc}</div>
                      </div>
                      <span className="rounded bg-emerald-500/10 px-2 py-1 text-xs text-emerald-700">
                        Available
                      </span>
                    </div>
                  ))}
                </div>
              </div>

              <div className="rounded-lg border border-amber-500/25 bg-amber-500/5 p-4">
                <div className="flex items-start gap-3">
                  <Shield className="mt-0.5 size-5 shrink-0 text-amber-600" />
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center justify-between">
                      <h3 className="text-sm font-semibold text-foreground">Risk notes</h3>
                      <span className="rounded bg-amber-500/10 px-2 py-1 text-xs text-amber-700">
                        Medium
                      </span>
                    </div>
                    <p className="mt-2 text-sm text-muted-foreground">
                      This server can read and write files on the local filesystem.
                    </p>
                    <Button variant="outline" size="sm" className="mt-3 h-8 rounded-md text-xs">
                      View security guide
                      <ExternalLink className="ml-2 size-3.5" />
                    </Button>
                  </div>
                </div>
              </div>
            </div>
          </section>
        </div>
      </div>
    </div>
  )
}

function ZapIcon(): React.JSX.Element {
  return (
    <span className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
      <Layers className="size-4" />
    </span>
  )
}
